import { mkdir, writeFile } from 'node:fs/promises'
import { dirname, resolve } from 'node:path'

const [port = '9374', outputDirectory = '.tmp'] = process.argv.slice(2)
const endpoint = `http://127.0.0.1:${port}`
const sleep = milliseconds => new Promise(resolvePromise => setTimeout(resolvePromise, milliseconds))

async function findMainTarget() {
  for (let attempt = 0; attempt < 40; attempt += 1) {
    const targets = await fetch(`${endpoint}/json`).then(response => response.json()).catch(() => [])
    for (const target of targets.filter(item => item.type === 'page')) {
      const connection = await connect(target.webSocketDebuggerUrl)
      const result = await connection.call('Runtime.evaluate', {
        expression: `!document.body.classList.contains('floating-window')`,
        returnByValue: true,
      }).catch(() => ({ result: { value: false } }))
      connection.socket.close()
      if (result.result.value) return target
    }
    await sleep(250)
  }
  throw new Error('没有找到 Token Manager 主窗口调试目标。')
}

async function connect(webSocketUrl) {
  const socket = new WebSocket(webSocketUrl)
  await new Promise((resolvePromise, reject) => {
    socket.addEventListener('open', resolvePromise, { once: true })
    socket.addEventListener('error', reject, { once: true })
  })
  let id = 0
  const pending = new Map()
  socket.addEventListener('message', event => {
    const message = JSON.parse(String(event.data))
    const request = pending.get(message.id)
    if (!request) return
    pending.delete(message.id)
    message.error ? request.reject(new Error(message.error.message)) : request.resolve(message.result)
  })
  const call = (method, params = {}) => new Promise((resolvePromise, reject) => {
    const messageId = ++id
    pending.set(messageId, { resolve: resolvePromise, reject })
    socket.send(JSON.stringify({ id: messageId, method, params }))
  })
  return { socket, call }
}

function intersection(first, second) {
  const width = Math.max(0, Math.min(first.right, second.right) - Math.max(first.left, second.left))
  const height = Math.max(0, Math.min(first.bottom, second.bottom) - Math.max(first.top, second.top))
  return width * height
}

const target = await findMainTarget()
const browser = await connect(target.webSocketDebuggerUrl)
await browser.call('Page.enable')
await browser.call('Runtime.enable')
const output = resolve(outputDirectory)
await mkdir(output, { recursive: true })
const results = []

for (const viewport of [
  { name: 'full', width: 1440, height: 900 },
  { name: 'half', width: 980, height: 760 },
]) {
  await browser.call('Emulation.setDeviceMetricsOverride', {
    width: viewport.width,
    height: viewport.height,
    deviceScaleFactor: 1,
    mobile: false,
  })
  await sleep(700)
  const measured = await browser.call('Runtime.evaluate', {
    expression: `(() => {
      const names=['header','.header-actions','.health-launch','.monitor-launch','.sync','.app-window-controls'];
      const rects={};
      for(const name of names){
        const node=name==='header'?document.querySelector('.content>header'):document.querySelector(name);
        if(!node)continue;
        const rect=node.getBoundingClientRect();
        rects[name]={left:rect.left,top:rect.top,right:rect.right,bottom:rect.bottom,width:rect.width,height:rect.height};
      }
      return {rects,scrollWidth:document.documentElement.scrollWidth,clientWidth:document.documentElement.clientWidth};
    })()`,
    returnByValue: true,
  })
  const value = measured.result.value
  if (!value.rects['.health-launch']) throw new Error(`${viewport.name} 窗口缺少监控健康入口。`)
  const controls = ['.health-launch', '.monitor-launch', '.sync']
  const overlaps = []
  for (let index = 0; index < controls.length; index += 1) {
    for (let next = index + 1; next < controls.length; next += 1) {
      const first = value.rects[controls[index]]
      const second = value.rects[controls[next]]
      if (first && second && intersection(first, second) > 0) overlaps.push(`${controls[index]} × ${controls[next]}`)
    }
  }
  const picture = await browser.call('Page.captureScreenshot', {
    format: 'png',
    fromSurface: true,
    captureBeyondViewport: false,
  })
  const outputPath = resolve(output, `health-${viewport.name}.png`)
  await writeFile(outputPath, Buffer.from(picture.data, 'base64'))
  results.push({ ...viewport, outputPath, ...value, overlaps })
}

await browser.call('Emulation.clearDeviceMetricsOverride')
browser.socket.close()
if (results.some(result => result.overlaps.length || result.scrollWidth > result.clientWidth)) {
  throw new Error(`监控健康入口响应式验收失败：${JSON.stringify(results)}`)
}
process.stdout.write(JSON.stringify(results, null, 2))
