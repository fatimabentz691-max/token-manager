import { spawn } from 'node:child_process'
import { existsSync } from 'node:fs'
import { mkdir, writeFile } from 'node:fs/promises'
import { join, resolve } from 'node:path'
import { preview } from 'vite'

// 设置页小窗验收：覆盖主窗口非最大化时最容易出现挤压的四档宽度。
const root = process.cwd()
const output = resolve(root, 'screenshots', 'floating-settings')
const profile = resolve(root, '.tmp', `floating-settings-${Date.now()}`)
await mkdir(output, { recursive: true })
await mkdir(profile, { recursive: true })

const edge = [
  join(process.env['PROGRAMFILES(X86)'] || '', 'Microsoft', 'Edge', 'Application', 'msedge.exe'),
  join(process.env.PROGRAMFILES || '', 'Microsoft', 'Edge', 'Application', 'msedge.exe'),
].find(existsSync)
if (!edge) throw new Error('找不到 Microsoft Edge')

const server = await preview({ root, configFile: false, preview: { host: '127.0.0.1', port: 4184, strictPort: true } })
const browser = spawn(edge, [
  '--headless=new',
  '--hide-scrollbars',
  '--remote-debugging-port=9344',
  `--user-data-dir=${profile}`,
  '--window-size=1280,900',
  'http://127.0.0.1:4184/',
], { windowsHide: true, stdio: 'ignore' })

const sleep = ms => new Promise(resolveSleep => setTimeout(resolveSleep, ms))
let target
for (let attempt = 0; attempt < 40; attempt++) {
  try {
    target = (await fetch('http://127.0.0.1:9344/json').then(response => response.json())).find(item => item.type === 'page')
    if (target) break
  } catch {}
  await sleep(200)
}
if (!target) throw new Error('无法连接 Edge 调试页')

const socket = new WebSocket(target.webSocketDebuggerUrl)
await new Promise((resolveOpen, rejectOpen) => {
  socket.addEventListener('open', resolveOpen, { once: true })
  socket.addEventListener('error', rejectOpen, { once: true })
})
let messageId = 0
const pending = new Map()
socket.addEventListener('message', event => {
  const message = JSON.parse(String(event.data))
  const request = pending.get(message.id)
  if (!request) return
  pending.delete(message.id)
  message.error ? request.reject(new Error(message.error.message)) : request.resolve(message.result)
})
const call = (method, params = {}) => new Promise((resolveCall, rejectCall) => {
  const id = ++messageId
  pending.set(id, { resolve: resolveCall, reject: rejectCall })
  socket.send(JSON.stringify({ id, method, params }))
})
const evaluate = expression => call('Runtime.evaluate', { expression, awaitPromise: true, returnByValue: true })

try {
  await call('Page.enable')
  await call('Runtime.enable')
  await sleep(700)
  await evaluate(`(()=>{const click=text=>[...document.querySelectorAll('button')].find(button=>button.textContent?.includes(text))?.click();click('设置');return true})()`)
  await sleep(250)
  await evaluate(`(()=>{[...document.querySelectorAll('.settings-nav button')].find(button=>button.textContent?.includes('悬浮窗'))?.click();return true})()`)
  await sleep(350)
  await evaluate(`document.querySelector('.floating-shape-picker button')?.click()`)
  await sleep(300)

  for (const width of [1280, 1040, 900, 760]) {
    await call('Emulation.setDeviceMetricsOverride', { width, height: 820, deviceScaleFactor: 1, mobile: false })
    await sleep(220)
    const audit = (await evaluate(`(()=>{
      const selectors=['.settings-center','.settings-nav','.settings-detail','.floating-control-card','.floating-settings-layout','.floating-shape-picker','.floating-preview-card','.floating-preview-window.v3']
      const boxes=Object.fromEntries(selectors.map(selector=>{const element=document.querySelector(selector);const rect=element?.getBoundingClientRect();return[selector,rect?{left:Math.round(rect.left),right:Math.round(rect.right),width:Math.round(rect.width),top:Math.round(rect.top),bottom:Math.round(rect.bottom)}:null]}))
      const visible=[...document.querySelectorAll('.settings-detail button,.settings-detail input,.settings-detail select')].filter(element=>{const style=getComputedStyle(element);const rect=element.getBoundingClientRect();return style.display!=='none'&&style.visibility!=='hidden'&&rect.width>0&&rect.height>0})
      return{width:innerWidth,overflowX:document.documentElement.scrollWidth>innerWidth,clippedControls:visible.filter(element=>{const rect=element.getBoundingClientRect();return rect.left<0||rect.right>innerWidth+1}).length,boxes}
    })()`)).result.value
    if (audit.overflowX || audit.clippedControls) throw new Error(`设置页 ${width}px 验收失败：${JSON.stringify(audit)}`)
    console.log(JSON.stringify(audit))
    const shot = await call('Page.captureScreenshot', { format: 'png', fromSurface: true, captureBeyondViewport: false })
    await writeFile(resolve(output, `floating-settings-${width}.png`), Buffer.from(shot.data, 'base64'))
  }
} finally {
  socket.close()
  browser.kill()
  await new Promise(resolveClose => server.httpServer.close(resolveClose))
}
