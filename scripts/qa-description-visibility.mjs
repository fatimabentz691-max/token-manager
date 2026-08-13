import { spawn } from 'node:child_process'
import { existsSync } from 'node:fs'
import { mkdir, writeFile } from 'node:fs/promises'
import { join, resolve } from 'node:path'
import { preview } from 'vite'

const root = process.cwd()
const output = resolve(root, 'artifacts', 'v0.11.6', 'description-visibility')
const profile = resolve(root, '.tmp', `description-visibility-${Date.now()}`)
await mkdir(output, { recursive: true })
await mkdir(profile, { recursive: true })
const edge = [
  join(process.env['PROGRAMFILES(X86)'] || '', 'Microsoft', 'Edge', 'Application', 'msedge.exe'),
  join(process.env.PROGRAMFILES || '', 'Microsoft', 'Edge', 'Application', 'msedge.exe'),
].find(existsSync)
if (!edge) throw new Error('找不到 Microsoft Edge')

const server = await preview({ root, configFile: false, preview: { host: '127.0.0.1', port: 4196, strictPort: true } })
const browser = spawn(edge, [
  '--headless=new', '--hide-scrollbars', '--remote-debugging-port=9396',
  `--user-data-dir=${profile}`, '--window-size=1440,900', 'http://127.0.0.1:4196/',
], { windowsHide: true, stdio: 'ignore' })
const sleep = ms => new Promise(resolveSleep => setTimeout(resolveSleep, ms))
let target
for (let attempt = 0; attempt < 40; attempt++) {
  try { target = (await fetch('http://127.0.0.1:9396/json').then(response => response.json())).find(item => item.type === 'page'); if (target) break } catch {}
  await sleep(200)
}
if (!target) throw new Error('无法连接 Edge 调试页')
const socket = new WebSocket(target.webSocketDebuggerUrl)
await new Promise((resolveOpen, rejectOpen) => { socket.addEventListener('open', resolveOpen, { once: true }); socket.addEventListener('error', rejectOpen, { once: true }) })
let id = 0
const pending = new Map()
socket.addEventListener('message', event => { const message = JSON.parse(String(event.data)); const request = pending.get(message.id); if (!request) return; pending.delete(message.id); message.error ? request.reject(new Error(message.error.message)) : request.resolve(message.result) })
const call = (method, params = {}) => new Promise((resolveCall, rejectCall) => { const callId = ++id; pending.set(callId, { resolve: resolveCall, reject: rejectCall }); socket.send(JSON.stringify({ id: callId, method, params })) })
const evaluate = expression => call('Runtime.evaluate', { expression, awaitPromise: true, returnByValue: true })

try {
  await call('Page.enable'); await call('Runtime.enable')
  await evaluate(`localStorage.setItem('token-manager-onboarding-v011','done');localStorage.setItem('token-manager-onboarding-v0113','done');localStorage.setItem('token-manager-description-visibility','hover');location.reload()`)
  await sleep(900)
  for (const [width, height] of [[1440, 900], [980, 760]]) {
    await call('Emulation.setDeviceMetricsOverride', { width, height, deviceScaleFactor: 1, mobile: false })
    await evaluate(`(()=>{const button=[...document.querySelectorAll('.shell>aside nav button')].find(item=>item.textContent?.trim()==='设置');button?.click();return Boolean(button)})()`)
    await sleep(350)
    for (const mode of ['hover', 'always', 'hidden']) {
      await evaluate(`localStorage.setItem('token-manager-description-visibility','${mode}');window.dispatchEvent(new StorageEvent('storage',{key:'token-manager-description-visibility',newValue:'${mode}'}))`)
      await sleep(250)
      await call('Input.dispatchMouseEvent', { type: 'mouseMoved', x: width - 2, y: height - 2 })
      const audit = (await evaluate(`(()=>{
        const nodes=[...document.querySelectorAll('.tm-supplemental-description')]
        const visible=nodes.filter(node=>{const style=getComputedStyle(node);return style.display!=='none'&&style.visibility!=='hidden'&&Number(style.opacity)>.5&&node.getBoundingClientRect().height>0})
        const status=document.querySelector('.header-copy [role="status"]')
        const statusStyle=status&&getComputedStyle(status)
        return {mode:document.documentElement.dataset.descriptionVisibility,total:nodes.length,visible:visible.length,statusVisible:Boolean(status&&statusStyle.display!=='none'&&statusStyle.visibility!=='hidden'&&status.getBoundingClientRect().height>0),overflowX:document.documentElement.scrollWidth>innerWidth}
      })()`)).result.value
      if (audit.total < 30 || audit.mode !== mode || audit.overflowX) throw new Error(`说明偏好基础验收失败：${JSON.stringify({ width, audit })}`)
      if (mode !== 'hidden' && !audit.statusVisible) throw new Error(`非隐藏模式下同步状态不可见：${JSON.stringify({ width, audit })}`)
      if (mode === 'hidden' && audit.statusVisible) throw new Error(`隐藏模式下同步状态仍常驻：${JSON.stringify({ width, audit })}`)
      if (mode === 'hover' && audit.visible !== 0) throw new Error(`靠近显示存在常驻说明：${JSON.stringify({ width, audit })}`)
      if (mode === 'always' && audit.visible !== audit.total) throw new Error(`始终显示未覆盖全部说明：${JSON.stringify({ width, audit })}`)
      if (mode === 'hidden' && audit.visible !== 0) throw new Error(`完全隐藏仍有说明可见：${JSON.stringify({ width, audit })}`)
      console.log(JSON.stringify({ width, ...audit }))
    }
    await evaluate(`localStorage.setItem('token-manager-description-visibility','hover');window.dispatchEvent(new StorageEvent('storage',{key:'token-manager-description-visibility',newValue:'hover'}))`)
    await sleep(200)
    const targetBox = (await evaluate(`(()=>{const group=document.querySelector('.settings-detail header.tm-supplemental-group');const rect=group?.getBoundingClientRect();return rect?{x:rect.left+Math.min(80,rect.width/2),y:rect.top+Math.min(35,rect.height/2)}:null})()`)).result.value
    if (!targetBox) throw new Error('找不到设置标题说明触发区')
    await call('Input.dispatchMouseEvent', { type: 'mouseMoved', x: targetBox.x, y: targetBox.y })
    await sleep(250)
    const hoverVisible = (await evaluate(`(()=>{const node=document.querySelector('.settings-detail header.tm-supplemental-group .tm-supplemental-description');if(!node)return false;const style=getComputedStyle(node);return style.visibility==='visible'&&Number(style.opacity)>.5&&node.getBoundingClientRect().height>0})()`)).result.value
    if (!hoverVisible) throw new Error(`靠近标题未显现说明：${width}px`)
    await evaluate(`localStorage.setItem('token-manager-description-visibility','hidden');window.dispatchEvent(new StorageEvent('storage',{key:'token-manager-description-visibility',newValue:'hidden'}))`)
    await sleep(200)
    const headerBox = (await evaluate(`(()=>{const header=document.querySelector('.content>header');const rect=header?.getBoundingClientRect();return rect?{x:rect.left+60,y:rect.top+40}:null})()`)).result.value
    if (!headerBox) throw new Error('找不到顶部同步状态触发区')
    await call('Input.dispatchMouseEvent', { type: 'mouseMoved', x: headerBox.x, y: headerBox.y })
    await sleep(250)
    const statusHoverVisible = (await evaluate(`(()=>{const node=document.querySelector('.header-copy [role="status"]');if(!node)return false;const style=getComputedStyle(node);return style.visibility==='visible'&&Number(style.opacity)>.5&&node.getBoundingClientRect().height>0})()`)).result.value
    if (!statusHoverVisible) throw new Error(`隐藏模式下靠近顶部未恢复同步状态：${width}px`)
    const shot = await call('Page.captureScreenshot', { format: 'png', fromSurface: true, captureBeyondViewport: false })
    await writeFile(resolve(output, `settings-hover-${width}.png`), Buffer.from(shot.data, 'base64'))
  }
} finally {
  socket.close(); browser.kill(); await new Promise(resolveClose => server.httpServer.close(resolveClose))
}
