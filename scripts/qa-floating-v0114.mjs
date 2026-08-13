import { spawn } from 'node:child_process'
import { mkdir, writeFile } from 'node:fs/promises'
import { existsSync } from 'node:fs'
import { join, resolve } from 'node:path'
import { preview } from 'vite'

const root = process.cwd()
const output = resolve(root, 'artifacts', 'v0.11.4', 'ui-check')
const profile = resolve(root, '.tmp', `floating-v0114-${Date.now()}`)
await mkdir(output, { recursive: true })
await mkdir(profile, { recursive: true })
const edge = [join(process.env['PROGRAMFILES(X86)'] || '', 'Microsoft', 'Edge', 'Application', 'msedge.exe'), join(process.env.PROGRAMFILES || '', 'Microsoft', 'Edge', 'Application', 'msedge.exe')].find(existsSync)
if (!edge) throw new Error('找不到 Microsoft Edge')
const server = await preview({ root, configFile: false, preview: { host: '127.0.0.1', port: 4194, strictPort: true } })
const browser = spawn(edge, ['--headless=new', '--hide-scrollbars', '--remote-debugging-port=9394', `--user-data-dir=${profile}`, '--window-size=980,760', 'http://127.0.0.1:4194/?floating=1'], { windowsHide: true, stdio: 'ignore' })
const sleep = ms => new Promise(resolveSleep => setTimeout(resolveSleep, ms))
let target
for (let attempt = 0; attempt < 40; attempt++) {
  try { target = (await fetch('http://127.0.0.1:9394/json').then(response => response.json())).find(item => item.type === 'page'); if (target) break } catch {}
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
  await evaluate(`localStorage.setItem('token-manager-theme','mono');localStorage.setItem('token-manager-description-visibility','hover')`)
  for (const [mode, width, height] of [['capsule',360,152],['compact',430,390],['full',760,720]]) {
    await call('Emulation.setDeviceMetricsOverride', { width, height, deviceScaleFactor: 1, mobile: false })
    await evaluate(`localStorage.setItem('token-manager-floating-config',JSON.stringify({version:3,mode:'${mode}',layout:'grid',enabled:['todayTokens','todayCalls','cacheChart','costChart','requestChart','tokenTrendChart'],order:['todayTokens','todayCalls','cacheChart','costChart','requestChart','tokenTrendChart'],mini:['todayTokens','todayCalls'],selectedKey:'__all__',expandedKeys:['__codex__'],interaction:'interactive',alwaysOnTop:true,snapToEdges:false,sizes:{capsule:{width:360,height:152},compact:{width:430,height:390},full:{width:760,height:720}},position:null}));location.reload()`)
    await sleep(900)
    const audit = (await evaluate(`(()=>{const root=document.querySelector('.floating-shell');const rect=root?.getBoundingClientRect();const tooltips=[...document.querySelectorAll('[role="tooltip"]')].filter(el=>getComputedStyle(el).display!=='none'&&getComputedStyle(el).visibility!=='hidden'&&Number(getComputedStyle(el).opacity)>0);const text=[...document.querySelectorAll('.floating-shell small,.floating-shell span')].filter(el=>{const size=parseFloat(getComputedStyle(el).fontSize);return size>0&&size<9});return{mode:root?.className,width:Math.round(rect?.width||0),height:Math.round(rect?.height||0),radius:root?getComputedStyle(root).borderRadius:'',overflowX:document.documentElement.scrollWidth>innerWidth,overflowY:document.documentElement.scrollHeight>innerHeight,visible:Boolean(rect&&rect.width>0&&rect.height>0),smallText:text.length,visibleTooltips:tooltips.length}})()`)).result.value
    const expectedRadius = mode === 'capsule' ? 28 : 24
    if (!audit.visible || audit.overflowX || audit.overflowY || Number.parseFloat(audit.radius) < expectedRadius || audit.smallText > 0 || audit.visibleTooltips > 1) throw new Error(`悬浮窗 ${mode} 验收失败：${JSON.stringify(audit)}`)
    if (mode === 'capsule') {
      await evaluate(`(()=>{document.querySelector('.capsule-summary .smooth-ring')?.dispatchEvent(new MouseEvent('mouseenter',{bubbles:true}));return true})()`)
      await sleep(220)
      const tooltipAudit = (await evaluate(`(()=>{const root=document.querySelector('.floating-shell')?.getBoundingClientRect();const tip=document.querySelector('.smooth-ring__tooltip')?.getBoundingClientRect();return{visible:Boolean(tip),inside:Boolean(root&&tip&&tip.left>=root.left&&tip.right<=root.right&&tip.top>=root.top&&tip.bottom<=root.bottom),root:root?{left:root.left,top:root.top,right:root.right,bottom:root.bottom}:null,tip:tip?{left:tip.left,top:tip.top,right:tip.right,bottom:tip.bottom}:null}})()`)).result.value
      if (!tooltipAudit.visible || !tooltipAudit.inside) throw new Error(`胶囊圆环提示层裁切验收失败：${JSON.stringify(tooltipAudit)}`)
      console.log(JSON.stringify({ tooltipAudit }))
      // 先离开圆环及其浮层，再进入文字触发区，模拟用户从窗口外靠近胶囊摘要。
      await call('Input.dispatchMouseEvent', { type: 'mouseMoved', x: 355, y: 4 })
      await sleep(320)
      const copyPoint = (await evaluate(`(()=>{const copy=document.querySelector('.capsule-summary .floating-copy');const rect=copy?.getBoundingClientRect();return rect?{x:rect.left+Math.min(40,rect.width/2),y:rect.top+rect.height/2}:null})()`)).result.value
      if (!copyPoint) throw new Error('找不到胶囊文字触发区')
      await call('Input.dispatchMouseEvent', { type: 'mouseMoved', x: copyPoint.x, y: copyPoint.y })
      await evaluate(`document.querySelector('.capsule-summary .floating-copy')?.dispatchEvent(new MouseEvent('mouseenter'))`)
      await sleep(220)
      const previewAudit = (await evaluate(`(()=>{const preview=document.querySelector('.capsule-preview');if(!preview)return null;const style=getComputedStyle(preview);const bg=style.backgroundColor.match(/[\d.]+/g)?.map(Number)||[];return{opacity:Number(style.opacity),background:style.backgroundColor,alpha:bg.length>3?bg[3]:1,backdrop:style.backdropFilter}})()`)).result.value
      if (!previewAudit || previewAudit.opacity < .99 || previewAudit.alpha < .99 || previewAudit.backdrop !== 'none') throw new Error(`胶囊详情浮层仍非实体背景：${JSON.stringify(previewAudit)}`)
      console.log(JSON.stringify({ previewAudit }))
    }
    console.log(JSON.stringify(audit))
    console.log(await capture(`${mode}.png`))
  }
} finally {
  socket.close(); browser.kill(); await new Promise(resolveClose => server.httpServer.close(resolveClose))
}
