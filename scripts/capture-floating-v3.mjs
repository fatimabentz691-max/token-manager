import { spawn } from 'node:child_process'
import { mkdir, writeFile } from 'node:fs/promises'
import { existsSync } from 'node:fs'
import { join, resolve } from 'node:path'
import { preview } from 'vite'

// 对 v0.8.5 经典窗口与灵动岛胶囊做同一套 DPI、溢出和截图验收。
const root = process.cwd()
const output = resolve(root, 'screenshots', 'floating-v085')
const profile = resolve(root, '.tmp', `floating-v085-${Date.now()}`)
await mkdir(output, { recursive: true })
await mkdir(profile, { recursive: true })
const edge = [join(process.env['PROGRAMFILES(X86)'] || '', 'Microsoft', 'Edge', 'Application', 'msedge.exe'), join(process.env.PROGRAMFILES || '', 'Microsoft', 'Edge', 'Application', 'msedge.exe')].find(existsSync)
if (!edge) throw new Error('找不到 Microsoft Edge')
const server = await preview({ root, configFile: false, preview: { host: '127.0.0.1', port: 4183, strictPort: true } })
const browser = spawn(edge, ['--headless=new', '--hide-scrollbars', '--remote-debugging-port=9343', `--user-data-dir=${profile}`, '--window-size=500,500', 'http://127.0.0.1:4183/?floating=1'], { windowsHide: true, stdio: 'ignore' })
const sleep = ms => new Promise(resolveSleep => setTimeout(resolveSleep, ms))
let target
for (let attempt = 0; attempt < 40; attempt++) {
  try { target = (await fetch('http://127.0.0.1:9343/json').then(response => response.json())).find(item => item.type === 'page'); if (target) break } catch {}
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
const capture = async name => { const shot = await call('Page.captureScreenshot', { format: 'png', fromSurface: true, captureBeyondViewport: false }); const path = resolve(output, name); await writeFile(path, Buffer.from(shot.data, 'base64')); return path }

try {
  await call('Page.enable'); await call('Runtime.enable')
  await evaluate(`localStorage.setItem('token-manager-theme','mono');localStorage.setItem('token-manager-floating-config',JSON.stringify({version:3,mode:'compact',layout:'grid',enabled:['todayTokens','todayCalls','cacheChart','costChart'],order:['todayTokens','todayCalls','cacheChart','costChart'],mini:['todayTokens','todayCalls'],selectedKey:'__all__',expandedKeys:[],interaction:'interactive',alwaysOnTop:true,snapToEdges:true,sizes:{capsule:{width:360,height:152},compact:{width:380,height:380},full:{width:380,height:380}},position:null}));location.reload()`)
  await sleep(900)
  for (const [mode, width, height] of [['compact',380,380],['capsule',360,152]]) {
    for (const scale of [1, 1.25, 1.5, 2]) {
      await call('Emulation.setDeviceMetricsOverride', { width, height, deviceScaleFactor: scale, mobile: false })
      await evaluate(`window.dispatchEvent(new CustomEvent('qa-set-floating-mode',{detail:'${mode}'}));document.querySelector('${mode === 'capsule' ? '[aria-label="折叠悬浮窗"]' : '[aria-label="展开悬浮窗"]'}')?.click()`)
      await sleep(300)
      const audit = (await evaluate(`(()=>{const root=document.querySelector('.floating-classic');const rect=root?.getBoundingClientRect();const content=document.querySelector('.floating-classic__content');return{mode:root?.className,dpr:devicePixelRatio,width:rect?.width,height:rect?.height,radius:root?getComputedStyle(root).borderRadius:'',overflowX:document.documentElement.scrollWidth>innerWidth,overflowY:document.documentElement.scrollHeight>innerHeight,contentScroll:content?content.scrollHeight>content.clientHeight:false,visible:Boolean(rect&&rect.width>0&&rect.height>0),buttons:[...document.querySelectorAll('button')].filter(button=>getComputedStyle(button).display!=='none').length}})()`)).result.value
      const radius = Number.parseFloat(audit.radius)
      const expectedRadius = mode === 'capsule' ? 28 : 24
      if (!audit.visible || audit.overflowX || audit.overflowY || radius < expectedRadius) throw new Error(`悬浮窗 ${mode} @${scale} 验收失败：${JSON.stringify({...audit,expectedRadius})}`)
      console.log(JSON.stringify(audit))
      if (scale === 1) console.log(await capture(`${mode}.png`))
    }
  }
} finally {
  socket.close(); browser.kill(); await new Promise(resolveClose => server.httpServer.close(resolveClose))
}
