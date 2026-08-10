import { spawn } from 'node:child_process'
import { mkdir, writeFile } from 'node:fs/promises'
import { existsSync } from 'node:fs'
import { join, resolve } from 'node:path'
import { preview } from 'vite'

const root = process.cwd()
const port = 4181
const debugPort = 9335
const outputDir = resolve(root, 'screenshots', 'edge-scroll')
const profileDir = resolve(root, '.tmp', `edge-scroll-${Date.now()}`)
await mkdir(outputDir, { recursive: true })
await mkdir(profileDir, { recursive: true })

const edge = [
  join(process.env['PROGRAMFILES(X86)'] || '', 'Microsoft', 'Edge', 'Application', 'msedge.exe'),
  join(process.env.PROGRAMFILES || '', 'Microsoft', 'Edge', 'Application', 'msedge.exe'),
].find(existsSync)
if (!edge) throw new Error('找不到 Microsoft Edge，无法执行界面截图验收。')

const server = await preview({ root, configFile: false, preview: { host: '127.0.0.1', port, strictPort: true } })
const browser = spawn(edge, [
  '--headless=new',
  `--remote-debugging-port=${debugPort}`,
  `--user-data-dir=${profileDir}`,
  '--window-size=1440,1000',
  '--force-device-scale-factor=1',
  `http://127.0.0.1:${port}/`,
], { windowsHide: true, stdio: 'ignore' })

const sleep = ms => new Promise(resolveSleep => setTimeout(resolveSleep, ms))
let targets
for (let attempt = 0; attempt < 40; attempt += 1) {
  try {
    targets = await fetch(`http://127.0.0.1:${debugPort}/json`).then(response => response.json())
    if (targets.some(target => target.type === 'page')) break
  } catch {}
  await sleep(250)
}
const target = targets?.find(item => item.type === 'page')
if (!target) throw new Error('无法连接 Edge 调试页面。')

const socket = new WebSocket(target.webSocketDebuggerUrl)
await new Promise((resolveOpen, rejectOpen) => {
  socket.addEventListener('open', resolveOpen, { once: true })
  socket.addEventListener('error', rejectOpen, { once: true })
})
let callId = 0
const pending = new Map()
socket.addEventListener('message', event => {
  const message = JSON.parse(String(event.data))
  if (!message.id || !pending.has(message.id)) return
  const request = pending.get(message.id)
  pending.delete(message.id)
  if (message.error) request.reject(new Error(message.error.message))
  else request.resolve(message.result)
})
function call(method, params = {}) {
  const id = ++callId
  socket.send(JSON.stringify({ id, method, params }))
  return new Promise((resolveCall, rejectCall) => pending.set(id, { resolve: resolveCall, reject: rejectCall }))
}
async function evaluate(expression) {
  const response = await call('Runtime.evaluate', { expression, awaitPromise: true, returnByValue: true })
  if (response.exceptionDetails) throw new Error(response.exceptionDetails.exception?.description || response.exceptionDetails.text)
  return response.result?.value
}
async function capture(name) {
  const result = await call('Page.captureScreenshot', { format: 'png', fromSurface: true, captureBeyondViewport: false })
  const path = resolve(outputDir, name)
  await writeFile(path, Buffer.from(result.data, 'base64'))
  return path
}

try {
  await call('Page.enable')
  await call('Runtime.enable')
  await evaluate(`localStorage.setItem('token-manager-theme','liquid-glass');localStorage.setItem('token-manager-liquid-tone','clear');localStorage.setItem('token-manager-selected-dashboard','__all__')`)
  await call('Page.reload', { ignoreCache: true })
  await sleep(1800)
  await evaluate(`[...document.querySelectorAll('.shell>aside nav button')].find(button=>button.textContent?.includes('\u6a21\u578b\u7528\u91cf'))?.click()`)
  await sleep(800)

  const brand = await evaluate(`(()=>{const label=document.querySelector('.brand>span');const version=document.querySelector('.brand>small');const labelBox=label?.getBoundingClientRect();const versionBox=version?.getBoundingClientRect();const style=label?getComputedStyle(label):null;return{text:label?.textContent?.trim(),labelWidth:labelBox?.width||0,scrollWidth:label?.scrollWidth||0,overflow:style?.overflow||'',textOverflow:style?.textOverflow||'',whiteSpace:style?.whiteSpace||'',versionTop:versionBox?.top||0,labelBottom:labelBox?.bottom||0}})()`)
  if (brand?.text !== 'Token Manager' || brand.scrollWidth > brand.labelWidth + 1 || brand.textOverflow === 'ellipsis' || brand.versionTop < brand.labelBottom - 1) {
    throw new Error(`左上角品牌名称仍未完整显示：${JSON.stringify(brand)}`)
  }

  const hidden = await evaluate(`(()=>{const rail=document.querySelector('.edge-scroll-rail');const track=document.querySelector('.edge-scroll-track');const thumb=document.querySelector('.edge-scroll-thumb');return{scrollable:rail?.classList.contains('is-scrollable'),trackOpacity:Number(getComputedStyle(track).opacity),thumbOpacity:Number(getComputedStyle(thumb).opacity),nativeWidth:getComputedStyle(document.documentElement).scrollbarWidth,documentScrollable:document.documentElement.scrollHeight>innerHeight}})()`)
  if (!hidden?.scrollable || !hidden.documentScrollable || hidden.trackOpacity !== 0 || hidden.thumbOpacity !== 0 || hidden.nativeWidth !== 'none') {
    throw new Error(`边缘滚动轨道默认隐藏状态异常：${JSON.stringify(hidden)}`)
  }
  console.log(await capture('scrollbar-hidden.png'))

  const hoverPoint = await evaluate(`(()=>{const rect=document.querySelector('.edge-scroll-rail')?.getBoundingClientRect();return{x:(rect?.left||innerWidth-18)+Math.max(1,(rect?.width||18)-3),y:Math.min(innerHeight-20,Math.max(80,(rect?.top||0)+180))}})()`)
  await call('Input.dispatchMouseEvent', { type: 'mouseMoved', x: hoverPoint.x, y: hoverPoint.y })
  await sleep(260)
  const revealed = await evaluate(`(()=>{const rail=document.querySelector('.edge-scroll-rail');const track=document.querySelector('.edge-scroll-track');const thumb=document.querySelector('.edge-scroll-thumb');const railRect=rail?.getBoundingClientRect();const thumbStyle=getComputedStyle(thumb);return{trackOpacity:Number(getComputedStyle(track).opacity),thumbOpacity:Number(thumbStyle.opacity),thumbRadius:thumbStyle.borderRadius,thumbWidth:thumbStyle.width,railRight:innerWidth-(railRect?.right||0),railWidth:railRect?.width||0}})()`)
  if (revealed?.trackOpacity < .9 || revealed.thumbOpacity < .9 || revealed.thumbRadius === '0px' || revealed.railRight !== 0 || revealed.railWidth < 16) {
    throw new Error(`鼠标到达最右侧后滚动轨道未正确显示：${JSON.stringify(revealed)}`)
  }
  console.log(await capture('scrollbar-revealed.png'))

  await call('Input.dispatchMouseEvent', { type: 'mouseMoved', x: 860, y: 430 })
  await sleep(260)
  const hiddenAgain = await evaluate(`Number(getComputedStyle(document.querySelector('.edge-scroll-thumb')).opacity)`)
  if (hiddenAgain !== 0) throw new Error(`鼠标离开最右侧后滚动轨道未隐藏：${hiddenAgain}`)
  console.log(JSON.stringify({ brand, hidden, revealed, hiddenAgain }))
} finally {
  socket.close()
  browser.kill()
  await new Promise(resolveClose => server.httpServer.close(resolveClose))
}
