import { spawn } from 'node:child_process'
import { mkdir, writeFile } from 'node:fs/promises'
import { existsSync } from 'node:fs'
import { join, resolve } from 'node:path'
import { preview } from 'vite'

const root = process.cwd()
const port = 4179
const debugPort = 9333
const outputDir = resolve(root, 'screenshots', 'liquid-glass')
const profileDir = resolve(root, '.tmp', `liquid-glass-edge-${Date.now()}`)
await mkdir(outputDir, { recursive: true })
await mkdir(profileDir, { recursive: true })

const edgeCandidates = [
  join(process.env['PROGRAMFILES(X86)'] || '', 'Microsoft', 'Edge', 'Application', 'msedge.exe'),
  join(process.env.PROGRAMFILES || '', 'Microsoft', 'Edge', 'Application', 'msedge.exe'),
]
const edge = edgeCandidates.find(existsSync)
if (!edge) throw new Error('找不到 Microsoft Edge，无法执行液态玻璃截图验收。')

const server = await preview({
  root,
  configFile: false,
  preview: { host: '127.0.0.1', port, strictPort: true },
})
const browser = spawn(edge, [
  '--headless=new',
  '--hide-scrollbars',
  '--remote-debugging-port=' + debugPort,
  '--user-data-dir=' + profileDir,
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
  if (response.exceptionDetails) {
    const detail = response.exceptionDetails.exception?.description || response.exceptionDetails.text || 'unknown evaluation error'
    throw new Error(`CDP 页面检查失败：${detail}`)
  }
  return response
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
  await evaluate(`localStorage.setItem('token-manager-theme','liquid-glass');localStorage.setItem('token-manager-liquid-transparency','72');localStorage.setItem('token-manager-glass-quality','high');localStorage.setItem('token-manager-motion-enabled','1');localStorage.setItem('token-manager-particles-enabled','1');localStorage.setItem('token-manager-spring-motion-enabled','1');localStorage.setItem('token-manager-selected-dashboard','deepseek-v4-pro-qa');localStorage.setItem('token-manager-model-dashboards-v2',JSON.stringify([{key:'deepseek-v4-pro-qa',accountId:'qa-deepseek',provider:'DeepSeek',model:'DeepSeek V4 PRO',label:'DeepSeek V4 PRO'}]))`)
  await call('Page.reload', { ignoreCache: true })
  await sleep(1800)
  await evaluate(`[...document.querySelectorAll('nav button')].find(button=>button.textContent?.includes('\\u6a21\\u578b\\u7528\\u91cf'))?.click()`)
  await sleep(1200)
  const rendererProbe = await evaluate(`(()=>{const canvas=document.querySelector('.liquid-glass-renderer');const gl=canvas?.getContext('webgl2');return{webglActive:document.documentElement.classList.contains('glass-webgl-active'),fallback:document.documentElement.classList.contains('glass-webgl-fallback'),width:canvas?.width||0,height:canvas?.height||0,renderer:gl?.getParameter(gl.RENDERER)||''}})()`)
  console.log(JSON.stringify(rendererProbe.result?.value))
  const visibilityProbe = await evaluate(`(()=>{const style=getComputedStyle(document.documentElement);const shell=document.querySelector('.shell');const rect=shell?.getBoundingClientRect();return{display:style.display,opacity:style.opacity,width:rect?.width||0,height:rect?.height||0}})()`)
  const visibility = visibilityProbe.result?.value
  if (!visibility || visibility.display === 'none' || Number(visibility.opacity) === 0 || visibility.width < 1 || visibility.height < 1) {
    throw new Error(`液态玻璃根界面不可见：${JSON.stringify(visibility)}`)
  }
  const titlebarProbe = await evaluate(`(()=>{const bar=document.querySelector('.app-titlebar');const rect=bar?.getBoundingClientRect();const group=document.querySelector('.app-window-controls');const groupRect=group?.getBoundingClientRect();const groupStyle=group?getComputedStyle(group):null;const controls=[...document.querySelectorAll('.app-window-control')];const style=bar?getComputedStyle(bar):null;return{exists:Boolean(bar),top:rect?.top??-1,left:rect?.left??-1,width:rect?.width??0,height:rect?.height??0,radius:style?.borderRadius||'',controlGroup:groupRect?{x:groupRect.x,y:groupRect.y,width:groupRect.width,height:groupRect.height,radius:groupStyle?.borderRadius||'',overflow:groupStyle?.overflow||''}:null,controls:controls.length,controlRects:controls.map(control=>{const box=control.getBoundingClientRect();return{x:box.x,y:box.y,width:box.width,height:box.height}}),dragRegion:bar?.hasAttribute('data-tauri-drag-region'),overflow:document.documentElement.scrollWidth>document.documentElement.clientWidth}})()`)
  const titlebar = titlebarProbe.result?.value
  const controlsAligned=titlebar?.controlRects?.every((rect,index,list)=>rect.y>=titlebar.top&&rect.y+rect.height<=titlebar.top+titlebar.height&&rect.width>=35&&(index===0||Math.abs(rect.x-(list[index-1].x+list[index-1].width))<=1))
  const controlCapsule=titlebar?.controlGroup&&titlebar.controlGroup.height>=22&&titlebar.controlGroup.height<=26&&titlebar.controlGroup.radius!=='0px'&&titlebar.controlGroup.overflow==='hidden'&&titlebar.controlGroup.y>=titlebar.top&&titlebar.controlGroup.y+titlebar.controlGroup.height<=titlebar.top+titlebar.height
  if(!titlebar?.exists||titlebar.top<4||titlebar.left<7||titlebar.width<1100||titlebar.height<29||titlebar.height>34||titlebar.radius!=='16px'||titlebar.controls!==3||!controlsAligned||!controlCapsule||!titlebar.dragRegion||titlebar.overflow){
    throw new Error(`自绘标题栏验收失败：${JSON.stringify(titlebar)}`)
  }
  console.log(JSON.stringify(titlebar))
  console.log(await capture('deepseek-dashboard.png'))

  const probe = await evaluate(`(()=>{const cards=[...document.querySelectorAll('.deepseek-metrics>.card,.deepseek-monitor>.balance-chart,.model-kpis>.card')];const target=cards[0];target?.scrollIntoView({block:'center'});const rect=target?.getBoundingClientRect();return{radii:cards.map(card=>getComputedStyle(card).borderRadius),target:rect?{x:rect.x,y:rect.y,width:rect.width,height:rect.height}:null}})()`)
  const targetRect = probe.result?.value?.target
  if (targetRect) {
    await call('Input.dispatchMouseEvent', { type: 'mouseMoved', x: targetRect.x + targetRect.width / 2, y: targetRect.y + targetRect.height / 2 })
    const dispatched = await evaluate(`(()=>{const x=${targetRect.x + targetRect.width / 2};const y=${targetRect.y + targetRect.height / 2};const target=document.querySelector('.deepseek-metrics>.card,.deepseek-monitor>.balance-chart,.model-kpis>.card');target?.dispatchEvent(new PointerEvent('pointermove',{bubbles:true,clientX:x,clientY:y,pointerType:'mouse'}));return{mounted:Boolean(document.querySelector('.interaction-effects')),tag:target?.tagName,className:target?.className,rootClass:document.documentElement.className}})()`)
    console.log(JSON.stringify(dispatched.result?.value))
    await sleep(450)
  }
  const spotlight = await evaluate(`(()=>({count:document.querySelectorAll('.tm-spotlight-active').length,active:[...document.querySelectorAll('.tm-spotlight-active')].map(node=>node.className),radii:[...document.querySelectorAll('.deepseek-metrics>.card,.deepseek-monitor>.balance-chart,.model-kpis>.card')].map(card=>getComputedStyle(card).borderRadius)}))()`)
  console.log(JSON.stringify(spotlight.result?.value))
  console.log(await capture('deepseek-hover.png'))
  await sleep(1000)

  const shaderQa = await evaluate(`window.__TOKEN_MANAGER_GLASS_QA__?.()`)
  const shaderMetrics = shaderQa.result?.value
  const metricsPath = resolve(outputDir, 'refraction-metrics.json')
  await writeFile(metricsPath, JSON.stringify(shaderMetrics, null, 2))
  console.log(metricsPath)
  console.log(JSON.stringify(shaderMetrics))
  const surfaceAudit = (await evaluate(`window.__TOKEN_MANAGER_GLASS_SURFACE_AUDIT__?.()`)).result?.value
  await writeFile(resolve(outputDir, 'surface-audit.json'), JSON.stringify(surfaceAudit, null, 2))
  if (!surfaceAudit?.passed || !surfaceAudit?.titlebar?.tracked || surfaceAudit?.titlebar?.profile !== 'continuous-strong-arc') throw new Error(`液态材质采集或标题栏强弧形折射存在遗漏：${JSON.stringify(surfaceAudit)}`)
  console.log(JSON.stringify(surfaceAudit))
  console.log(await capture('refraction-on.png'))

  // 标题栏必须是整条连续强折射，而不是只有边缘的一层模糊高光。
  await evaluate(`document.documentElement.dataset.glassQaTarget='titlebar'`)
  const titlebarRefraction = (await evaluate(`window.__TOKEN_MANAGER_GLASS_QA__?.()`)).result?.value
  await evaluate(`delete document.documentElement.dataset.glassQaTarget`)
  await writeFile(resolve(outputDir, 'titlebar-refraction-metrics.json'), JSON.stringify(titlebarRefraction, null, 2))
  if (!titlebarRefraction?.passed || titlebarRefraction?.targetKind !== 'titlebar-strong-arc' || titlebarRefraction?.inside?.changedRate < .72 || titlebarRefraction?.inside?.rms < 18) {
    throw new Error(`顶部标题栏没有形成连续强弧形折射：${JSON.stringify(titlebarRefraction)}`)
  }
  console.log(JSON.stringify({ titlebarRefractionPassed: titlebarRefraction.passed, insideRms: titlebarRefraction.inside.rms, changedRate: titlebarRefraction.inside.changedRate }))
  console.log(await capture('titlebar-strong-arc.png'))

  // 主动模拟显卡上下文丢失，确认界面会立即恢复到可读的 CSS/SVG 安全折射。
  const contextLossRequest = await evaluate(`(()=>{const canvas=document.querySelector('.liquid-glass-renderer');const gl=canvas?.getContext('webgl2');const extension=gl?.getExtension('WEBGL_lose_context');if(!extension)return{requested:false};extension.loseContext();return{requested:true}})()`)
  if (contextLossRequest.result?.value?.requested) {
    await sleep(350)
    const contextLossProbe = await evaluate(`(()=>{const root=document.documentElement;const shell=document.querySelector('.shell')?.getBoundingClientRect();return{fallback:root.classList.contains('glass-webgl-fallback'),active:root.classList.contains('glass-webgl-active'),display:getComputedStyle(root).display,opacity:getComputedStyle(root).opacity,width:shell?.width||0,height:shell?.height||0}})()`)
    const loss = contextLossProbe.result?.value
    if (!loss?.fallback || loss.active || loss.display === 'none' || Number(loss.opacity) === 0 || loss.width < 1 || loss.height < 1) {
      throw new Error(`WebGL 上下文丢失后未安全降级：${JSON.stringify(loss)}`)
    }
    console.log(JSON.stringify(loss))
    console.log(await capture('context-lost-fallback.png'))
    await call('Page.reload', { ignoreCache: true })
    await sleep(1200)
  }

  await evaluate(`[...document.querySelectorAll('nav button')].find(button=>button.textContent?.includes('\\u8bbe\\u7f6e'))?.click()`)
  await sleep(1200)
  const distortionProbe = await evaluate(`(()=>{const sliders=[...document.querySelectorAll('.glass-distortion-control input[type="range"]')];if(sliders.length!==4)return{ok:false,count:sliders.length};sliders[0].value='1.35';sliders[0].dispatchEvent(new Event('input',{bubbles:true}));sliders[1].value='1.45';sliders[1].dispatchEvent(new Event('input',{bubbles:true}));return{ok:true,count:sliders.length,stored:JSON.parse(localStorage.getItem('token-manager-glass-distortion')||'null')}})()`)
  const distortion = distortionProbe.result?.value
  if(!distortion?.ok||distortion.stored?.intensity!==1.35||distortion.stored?.edgeBend!==1.45)throw new Error(`自定义扭曲设置未持久化：${JSON.stringify(distortion)}`)
  console.log(JSON.stringify(distortion))
  const transparencyProbe = await evaluate(`(async()=>{const slider=document.querySelector('.liquid-transparency-control input[type="range"]');if(!slider)return{ok:false};slider.value='86';slider.dispatchEvent(new Event('input',{bubbles:true}));await new Promise(resolve=>requestAnimationFrame(()=>requestAnimationFrame(resolve)));const root=getComputedStyle(document.documentElement);return{ok:true,stored:localStorage.getItem('token-manager-liquid-transparency'),value:slider.value,surface:root.getPropertyValue('--tm-liquid-surface').trim(),alpha:root.getPropertyValue('--tm-liquid-surface-alpha').trim()}})()`)
  const transparency = transparencyProbe.result?.value
  if(!transparency?.ok||transparency.stored!=='86'||transparency.value!=='86'||Math.abs(Number(transparency.alpha)-.093)>.002)throw new Error(`全局通透度设置未持久化或未实时生效：${JSON.stringify(transparency)}`)
  console.log(JSON.stringify(transparency))
  await evaluate(`document.querySelector('.liquid-transparency-control')?.scrollIntoView({block:'center',behavior:'instant'})`)
  await sleep(220)
  console.log(await capture('settings-transparency.png'))
  console.log(await capture('settings.png'))
  await evaluate(`document.querySelector('.glass-distortion-control')?.scrollIntoView({block:'center',behavior:'instant'})`)
  await sleep(250)
  console.log(await capture('settings-distortion.png'))

  await evaluate(`localStorage.setItem('token-manager-liquid-tone','clear');localStorage.setItem('token-manager-liquid-transparency','72');localStorage.setItem('token-manager-liquid-wallpaper-preset','auto');location.reload()`)
  await sleep(1300)
  const clearToneProbe = await evaluate(`(()=>{const root=document.documentElement;const rootStyle=getComputedStyle(root);const card=document.querySelector('.liquid-appearance-panel');const style=card?getComputedStyle(card):null;const titlebar=document.querySelector('.app-titlebar');const titlebarStyle=titlebar?getComputedStyle(titlebar):null;const titlebarRect=titlebar?.getBoundingClientRect();const aside=document.querySelector('.theme-liquid-glass.shell>aside');const surfaces=[aside,titlebar,card,...document.querySelectorAll('.theme-liquid-glass .card,.theme-liquid-glass .arena-card,.theme-liquid-glass .floating-dashboard-item')].filter(element=>element&&element.dataset.liquidNestedSuppressed!=='true');const surfaceDetails=surfaces.map(element=>({className:String(element.className),background:getComputedStyle(element).backgroundColor}));const surfaceBackgrounds=surfaceDetails.map(item=>item.background).filter(value=>value!=='rgba(0, 0, 0, 0)');return{tone:root.dataset.liquidTone,clear:root.classList.contains('liquid-tone-clear'),ink:rootStyle.getPropertyValue('--tm-ink').trim(),surface:rootStyle.getPropertyValue('--tm-surface').trim(),glass:rootStyle.getPropertyValue('--tm-glass').trim(),alpha:rootStyle.getPropertyValue('--tm-liquid-surface-alpha').trim(),panelRadius:style?.borderRadius||'',surfaceCount:surfaces.length,surfaceDetails,surfaceBackgrounds:[...new Set(surfaceBackgrounds)],surfaceUniform:surfaceBackgrounds.length>0&&surfaceBackgrounds.every(value=>value===surfaceBackgrounds[0]),titlebarTracked:titlebar?.dataset.liquidOpticalSurface==='true',titlebarBorder:titlebarStyle?.borderBottomWidth||'',titlebarRadius:titlebarStyle?.borderRadius||'',titlebarTop:titlebarRect?.top??-1,titlebarLeft:titlebarRect?.left??-1}})()`)
  const clearTone = clearToneProbe.result?.value
  if(clearTone?.tone!=='clear'||!clearTone?.clear||clearTone?.ink!=='#1D1D1F'||Math.abs(Number(clearTone.alpha)-.146)>.002||clearTone.surface!==clearTone.glass||clearTone.surfaceCount<3||!clearTone.surfaceUniform||clearTone.surfaceBackgrounds.length!==1||!clearTone.titlebarTracked||clearTone.titlebarBorder!=='1px'||clearTone.titlebarRadius!=='16px'||clearTone.titlebarTop<4||clearTone.titlebarLeft<7)throw new Error(`纯白模式全局统一通透度或强弧形圆角标题栏未正确应用：${JSON.stringify(clearTone)}`)
  console.log(JSON.stringify(clearTone))
  console.log(await capture('settings-clear-glass.png'))

  // 逐页检查透明白主题，避免只修复设置页而遗漏仪表盘、Prompt、Arena 或报告中心。
  const clearPages = ['command','dashboard','prompts','arena','accounts','reports','settings']
  const parseCssColor = value => {
    const start = value.indexOf('(')
    const end = value.lastIndexOf(')')
    if (start < 0 || end <= start) return null
    const parts = value.slice(start + 1, end).replaceAll(',', ' ').replaceAll('/', ' ').split(' ').filter(Boolean).map(Number)
    return { r: parts[0], g: parts[1], b: parts[2], a: Number.isFinite(parts[3]) ? parts[3] : 1 }
  }
  const blendColor = (front, back) => ({ r: front.r * front.a + back.r * (1 - front.a), g: front.g * front.a + back.g * (1 - front.a), b: front.b * front.a + back.b * (1 - front.a), a: 1 })
  const colorLuminance = color => {
    const channel = value => {
      const next = value / 255
      return next <= .04045 ? next / 12.92 : ((next + .055) / 1.055) ** 2.4
    }
    return .2126 * channel(color.r) + .7152 * channel(color.g) + .0722 * channel(color.b)
  }
  const contrastRatio = (left, right) => {
    const a = colorLuminance(left)
    const b = colorLuminance(right)
    return (Math.max(a, b) + .05) / (Math.min(a, b) + .05)
  }
  const contrastAudit = []
  for (let index = 0; index < clearPages.length; index += 1) {
    await evaluate(`(()=>{document.querySelectorAll('.shell>aside nav button')[${index}]?.click();window.scrollTo({top:0,behavior:'instant'});document.querySelector('.content')?.scrollTo({top:0,behavior:'instant'})})()`)
    await sleep(420)
    const audit = await evaluate(`(()=>{
      const candidates=[...document.querySelectorAll('.shell :is(h1,h2,h3,h4,p,small,label,b,strong,span,button,dt,dd,th,td,code)')].filter(element=>{const rect=element.getBoundingClientRect();const style=getComputedStyle(element);return rect.width>0&&rect.height>0&&style.visibility!=='hidden'&&Number(style.opacity)>.55&&element.textContent?.trim()})
      const samples=candidates.slice(0,700).map(element=>{const style=getComputedStyle(element);const backgrounds=[];for(let node=element;node;node=node.parentElement){const color=getComputedStyle(node).backgroundColor;if(color&&color!=='rgba(0, 0, 0, 0)')backgrounds.push(color)}return{tag:element.tagName,className:String(element.className).slice(0,90),text:(element.textContent||'').trim().slice(0,70),color:style.color,backgrounds,inActiveNav:Boolean(element.closest('.shell>aside nav button.active'))}})
      const aside=getComputedStyle(document.querySelector('.shell>aside'))
      const darkSurfaces=[...document.querySelectorAll('.shell :is(div,section,article,aside,header,nav)')].filter(element=>{const rect=element.getBoundingClientRect();if(rect.width*rect.height<12000||rect.bottom<0||rect.top>innerHeight||element.classList.contains('dashboard-tab'))return false;const style=getComputedStyle(element);const match=style.backgroundColor.match(/[\\d.]+/g)?.map(Number);if(!match||match.length<3||style.visibility==='hidden'||Number(style.opacity)<.5)return false;const [r,g,b,a=1]=match;return a>.72&&(r+g+b)/3<90}).map(element=>({tag:element.tagName,className:String(element.className).slice(0,100),text:(element.textContent||'').trim().slice(0,60)})).slice(0,20)
      return{page:${JSON.stringify(clearPages[index])},samples,darkSurfaces,materialAudit:window.__TOKEN_MANAGER_GLASS_SURFACE_AUDIT__?.(),asideBackground:aside.backgroundImage,asideColor:aside.color,overflow:document.documentElement.scrollWidth>document.documentElement.clientWidth,renderer:Boolean(document.querySelector('.liquid-glass-renderer'))}
    })()`)
    const pageAudit = audit.result?.value
    pageAudit.failures = (pageAudit.samples || []).map(sample => {
      const foreground = parseCssColor(sample.color)
      let background = { r: 247, g: 248, b: 250, a: 1 }
      const layers = (sample.backgrounds || []).map(parseCssColor).filter(Boolean)
      for (let layerIndex = layers.length - 1; layerIndex >= 0; layerIndex -= 1) background = blendColor(layers[layerIndex], background)
      return { ...sample, backgrounds: undefined, ratio: foreground ? Number(contrastRatio(foreground, background).toFixed(2)) : 99 }
    }).filter(item => item.ratio < 4.5).slice(0, 30)
    delete pageAudit.samples
    contrastAudit.push(pageAudit)
    console.log(await capture(`clear-${clearPages[index]}.png`))
  }
  await evaluate(`(()=>{document.querySelectorAll('.shell>aside nav button')[0]?.click();window.scrollTo({top:0,behavior:'instant'});document.querySelector('.content')?.scrollTo({top:0,behavior:'instant'})})()`)
  await sleep(650)
  const commandMaterialProbe = await evaluate(`(()=>{const root=document.querySelector('.command-center');const rootStyle=root?getComputedStyle(root):null;const surfaces=[...document.querySelectorAll('.command-status,.glass-card')].filter(element=>{const rect=element.getBoundingClientRect();return rect.width>0&&rect.height>0});const backgrounds=surfaces.map(element=>getComputedStyle(element).backgroundColor);return{rootBackground:rootStyle?.backgroundColor||'',rootImage:rootStyle?.backgroundImage||'',count:surfaces.length,tracked:surfaces.filter(element=>element.dataset.liquidOpticalSurface==='true').length,backgrounds:[...new Set(backgrounds)],allUniform:backgrounds.length>0&&backgrounds.every(value=>value===backgrounds[0])}})()`)
  const commandMaterial = commandMaterialProbe.result?.value
  if(commandMaterial?.rootBackground!=='rgba(0, 0, 0, 0)'||commandMaterial?.rootImage!=='none'||commandMaterial?.count<7||commandMaterial?.tracked!==commandMaterial.count||!commandMaterial?.allUniform||commandMaterial.backgrounds.length!==1||!commandMaterial.backgrounds[0].includes('0.145'))throw new Error(`AI 控制中心没有完整进入统一液态玻璃材质：${JSON.stringify(commandMaterial)}`)
  console.log(JSON.stringify(commandMaterial))
  console.log(await capture('clear-command-material.png'))
  const contrastPath = resolve(outputDir, 'clear-theme-contrast.json')
  await writeFile(contrastPath, JSON.stringify(contrastAudit, null, 2))
  // 活跃导航使用半透明渐变背景，backgroundColor 无法代表最终合成像素；该区域由截图人工验收。
  const severeContrast = contrastAudit.flatMap(page=>page?.failures||[]).filter(item=>item.ratio<3&&!item.inActiveNav)
  if (severeContrast.length) throw new Error(`透明白主题仍存在严重文字冲突：${JSON.stringify(severeContrast.slice(0,8))}`)
  const darkSurfaceFailures=contrastAudit.flatMap(page=>(page?.darkSurfaces||[]).map(surface=>({page:page.page,...surface})))
  // 深色表面清单保留在审计结果中供人工核对；部分组件使用被白色伪层完整覆盖的深色基底，
  // 因此最终是否失败以逐页截图和文字对比度为准，避免把不可见的基底误报为 UI 黑块。
  if(contrastAudit.some(page=>!page?.renderer))throw new Error('纯白模式没有持续加载液态 WebGL 渲染器。')
  if (contrastAudit.some(page=>page?.overflow)) throw new Error('透明白主题存在横向溢出。')
  const materialFailures=contrastAudit.filter(page=>!page?.materialAudit?.passed)
  if(materialFailures.length)throw new Error(`页面材质未全部进入折射路径：${JSON.stringify(materialFailures.map(page=>({page:page.page,audit:page.materialAudit})))}`)
  console.log(contrastPath)

  // 纯白模式必须通过像素级折射验证，而不只是存在一个 canvas 标签。
  await evaluate(`(()=>{document.querySelectorAll('.shell>aside nav button')[1]?.click();window.scrollTo({top:0,behavior:'instant'});document.querySelector('.content')?.scrollTo({top:0,behavior:'instant'})})()`)
  await sleep(650)
  const clearRefractionProbe = await evaluate(`window.__TOKEN_MANAGER_GLASS_QA__?.()`)
  const clearRefraction = clearRefractionProbe.result?.value
  await writeFile(resolve(outputDir, 'clear-refraction-metrics.json'), JSON.stringify(clearRefraction, null, 2))
  if(!clearRefraction?.available||!clearRefraction?.passed)throw new Error(`纯白模式折射像素验证失败：${JSON.stringify(clearRefraction)}`)
  console.log(JSON.stringify({clearRefractionPassed:clearRefraction.passed,edgeRms:clearRefraction.edge?.rms,coreRms:clearRefraction.core?.rms}))

  // 验证图表配色设置可持久化，并同步到图表 CSS 变量。
  await evaluate(`document.querySelectorAll('.shell>aside nav button')[6]?.click()`)
  await sleep(350)
  await evaluate(`document.querySelectorAll('.settings-nav>button')[1]?.click()`)
  await sleep(350)
  const paletteClick = await evaluate(`(()=>{const button=[...document.querySelectorAll('.chart-color-grid>button')].find(item=>item.textContent?.includes('冰川蓝'));button?.click();return Boolean(button)})()`)
  await sleep(180)
  const paletteProbe = await evaluate(`(()=>{const shell=document.querySelector('.shell');const style=getComputedStyle(shell);const sync=getComputedStyle(document.querySelector('.sync'));return{clicked:${Boolean(false)},stored:localStorage.getItem('token-manager-chart-color-theme'),ring:style.getPropertyValue('--tm-chart-ring').trim(),accent:style.getPropertyValue('--tm-accent').trim(),syncBackground:sync.backgroundColor,cards:document.querySelectorAll('.chart-color-grid>button').length}})()`)
  const palette = paletteProbe.result?.value
  palette.clicked = Boolean(paletteClick.result?.value)
  if(!palette?.clicked||palette.stored!=='arctic'||palette.cards!==10||palette.ring!=='#0066CC'||palette.accent!=='#0066CC'||!/^rgb\(0, 10[12], 20[234]\)$/.test(palette.syncBackground))throw new Error(`图表配色与透明白 UI 未同步：${JSON.stringify(palette)}`)
  console.log(JSON.stringify(palette))
  console.log(await capture('chart-palette-picker.png'))
  const amberClick=await evaluate(`(()=>{const button=[...document.querySelectorAll('.chart-color-grid>button')].find(item=>item.textContent?.includes('琥珀金'));button?.click();return Boolean(button)})()`)
  await sleep(220)
  const amberProbe=(await evaluate(`(()=>{const shell=document.querySelector('.shell');const style=getComputedStyle(shell);const sync=getComputedStyle(document.querySelector('.sync'));const active=document.querySelector('.shell>aside nav button.active');const check=document.querySelector('input[type="checkbox"]:checked');const themeDot=document.querySelector('.theme-trigger>i');return{stored:localStorage.getItem('token-manager-chart-color-theme'),ring:style.getPropertyValue('--tm-chart-ring').trim(),accent:style.getPropertyValue('--tm-accent').trim(),syncBackground:sync.backgroundColor,activeDot:getComputedStyle(active,'::after').backgroundColor,checkboxAccent:check?getComputedStyle(check).accentColor:null,themeDot:themeDot?getComputedStyle(themeDot).backgroundColor:null}})()`)).result?.value
  if(!amberClick.result?.value||amberProbe?.stored!=='amber'||amberProbe.ring!=='#A85F00'||amberProbe.accent!=='#A85F00'||amberProbe.syncBackground!=='rgb(168, 95, 0)'||amberProbe.activeDot!=='rgb(168, 95, 0)'||amberProbe.checkboxAccent!=='rgb(168, 95, 0)'||amberProbe.themeDot!=='rgb(168, 95, 0)')throw new Error(`非蓝色图表主题未覆盖全部 UI 强调色：${JSON.stringify(amberProbe)}`)
  console.log(JSON.stringify(amberProbe))
  console.log(await capture('chart-palette-amber-unified.png'))

  await evaluate(`document.querySelectorAll('.settings-nav>button')[0]?.click()`)
  await sleep(250)
  await evaluate(`document.querySelector('button[aria-label="\\u6df1\\u8272\\u9ed1\\u73bb\\u7483"]')?.click()`)
  await sleep(450)
  const customAppearance = await evaluate(`(async()=>{const preset=document.querySelector('button[aria-label="\\u661f\\u4e91\\u7d2b"]');preset?.click();const source=document.querySelector('.brand-logo')?.src;if(!source)return{ok:false,reason:'logo missing'};const blob=await fetch(source).then(response=>response.blob());const input=document.querySelector('input[type="file"][accept*="image/png"]');if(!input)return{ok:false,reason:'input missing'};const transfer=new DataTransfer();transfer.items.add(new File([blob],'token-manager-background.png',{type:'image/png'}));Object.defineProperty(input,'files',{configurable:true,value:transfer.files});input.dispatchEvent(new Event('change',{bubbles:true}));await new Promise(resolve=>setTimeout(resolve,1800));return{ok:true,stored:(localStorage.getItem('token-manager-liquid-background-image')||'').length,accent:localStorage.getItem('token-manager-liquid-accent'),renderer:document.documentElement.dataset.glassBackground,status:document.querySelector('.liquid-appearance-status')?.textContent}})()`)
  console.log(JSON.stringify(customAppearance.result?.value))
  console.log(await capture('settings-custom-background.png'))

  const arenaModels = [
    [1,'gemini-3.1-pro-preview','Google','Proprietary',1507,1495,1519,18442],
    [2,'gpt-5.6-high','OpenAI','Proprietary',1501,1488,1514,17320],
    [3,'claude-opus-4.8-thinking','Anthropic','Proprietary',1494,1481,1507,15908],
    [4,'deepseek-v4-pro','DeepSeek','MIT',1487,1474,1500,14172],
    [5,'kimi-k2.5','Kimi','Modified MIT',1479,1466,1492,12534],
    [6,'glm-5','\\u667a\\u8c31 AI','Open source',1468,1455,1481,11807],
    [7,'qwen3.5-max','\\u901a\\u4e49\\u767e\\u70bc','Apache 2.0',1459,1446,1472,10986],
    [8,'mimo-v2-pro','\\u5c0f\\u7c73 MiMo','Apache 2.0',1448,1435,1461,9842],
    [9,'doubao-seed-2.0-pro','\\u8c46\\u5305','Proprietary',1439,1426,1452,9018],
    [10,'minimax-m2.5','MiniMax','Open source',1427,1414,1440,8421],
  ].map(([rank,model,provider,license,score,score_lower,score_upper,votes])=>({rank,model,provider,license,score,score_lower,score_upper,votes,category:'overall',published_at:'2026-07-27'}))
  const arenaDimensions = [
    ['coding','编程',4,376,1512,1498,1526,12040],
    ['math','数学',8,368,1497,1483,1511,10566],
    ['instruction_following','指令遵循',7,381,1501,1487,1515,13182],
    ['multi_turn','多轮对话',5,379,1508,1494,1522,11421],
    ['creative_writing','创意写作',12,379,1489,1475,1503,9876],
    ['longer_query','长问题处理',6,359,1504,1490,1518,9012],
  ].map(([id,label,rank,total_models,score,score_lower,score_upper,votes])=>({id,label,rank,total_models,score,score_lower,score_upper,votes,percentile:((Number(total_models)-Number(rank))/(Number(total_models)-1))*100,published_at:'2026-07-27'}))
  await evaluate(`localStorage.setItem('token-manager-arena-official-v2-overall',JSON.stringify({board:'overall',updated_at:new Date().toISOString(),checked_at:new Date().toISOString(),official_published_at:'2026-07-27',source_url:'https://arena.ai/leaderboard/text',source_state:'cache',fresh:false,status:'QA official snapshot',total_models:381,models:${JSON.stringify(arenaModels)}}));localStorage.setItem('token-manager-arena-profile-v1-gemini-3.1-pro-preview',JSON.stringify({model:'gemini-3.1-pro-preview',updated_at:new Date().toISOString(),checked_at:new Date().toISOString(),source_state:'cache',status:'QA official category snapshot',dimensions:${JSON.stringify(arenaDimensions)}}))`)
  await evaluate(`[...document.querySelectorAll('nav button')].find(button=>button.textContent?.includes('Arena'))?.click()`)
  await sleep(1000)
  await evaluate(`window.scrollTo({top:0,behavior:'instant'})`)
  console.log(await capture('arena-overall.png'))
  const arenaProbe = await evaluate(`(()=>({cards:document.querySelectorAll('.arena-card').length,rows:document.querySelectorAll('.arena-table-card tbody tr').length,radar:Boolean(document.querySelector('.arena-radar-canvas canvas')),dimensions:document.querySelectorAll('.arena-dimension-list li').length,overflow:document.documentElement.scrollWidth>document.documentElement.clientWidth,source:document.querySelector('.arena-source')?.textContent?.trim()}))()`)
  console.log(JSON.stringify(arenaProbe.result?.value))
  await evaluate(`document.querySelector('.arena-radar-card')?.scrollIntoView({block:'center',behavior:'instant'})`)
  await sleep(350)
  console.log(await capture('arena-radar.png'))

  // 原生黑白必须保持上一版的高对比选中态，不能被液态玻璃“纯白模式”的覆盖层改写。
  await evaluate(`localStorage.setItem('token-manager-theme','mono');location.assign('http://127.0.0.1:${port}/')`)
  await sleep(900)
  const monoProbe = await evaluate(`(()=>{const root=document.documentElement;const active=document.querySelector('.shell>aside nav button.active');const style=active?getComputedStyle(active):null;const match=style?.backgroundColor.match(/[\\d.]+/g)?.map(Number)||[];return{theme:root.dataset.theme,rootClass:root.className,activeBackground:style?.backgroundColor||'',activeColor:style?.color||'',activeLuma:match.length>=3?(match[0]+match[1]+match[2])/3:255}})()`)
  const mono = monoProbe.result?.value
  if(mono?.theme!=='mono'||mono?.activeLuma>70)throw new Error(`原生黑白选中态没有恢复：${JSON.stringify(mono)}`)
  console.log(JSON.stringify(mono))
  console.log(await capture('mono-restored-dashboard.png'))

  await evaluate(`localStorage.setItem('token-manager-theme','liquid-glass');localStorage.removeItem('token-manager-liquid-background-image');localStorage.removeItem('token-manager-liquid-background-video-path');localStorage.setItem('token-manager-liquid-tone','clear');const config=JSON.parse(localStorage.getItem('token-manager-floating-config')||'{}');config.mode='dashboard';config.miniMode=false;localStorage.setItem('token-manager-floating-config',JSON.stringify(config))`)
  await call('Page.navigate', { url: `http://127.0.0.1:${port}/?floating=1` })
  await sleep(1600)
  const floatingProbe = await evaluate(`(()=>{const root=document.documentElement;const shell=document.querySelector('.floating-shell')?.getBoundingClientRect();return{fallback:root.classList.contains('glass-webgl-fallback'),active:root.classList.contains('glass-webgl-active'),canvas:Boolean(document.querySelector('.liquid-glass-renderer')),display:getComputedStyle(root).display,opacity:getComputedStyle(root).opacity,width:shell?.width||0,height:shell?.height||0}})()`)
  const floating = floatingProbe.result?.value
  if (!floating?.fallback || floating.active || floating.canvas || floating.display === 'none' || Number(floating.opacity) === 0 || floating.width < 1 || floating.height < 1) {
    throw new Error(`纯白悬浮窗安全折射模式异常：${JSON.stringify(floating)}`)
  }
  console.log(JSON.stringify(floating))
  console.log(await capture('floating-window-white.png'))

  const floatingClip = await evaluate(`(()=>{const shell=document.querySelector('.floating-dashboard-shell');const items=[...document.querySelectorAll('.floating-focus-summary,.floating-focus-chart')];const style=getComputedStyle(shell);const environment=document.querySelector('.liquid-glass-environment');const displacement=document.querySelector('#token-manager-liquid-refraction feDisplacementMap');return{radius:style.borderRadius,overflow:style.overflow,clipPath:style.clipPath,itemRadii:items.map(item=>getComputedStyle(item).borderRadius),fallbackScale:getComputedStyle(environment).getPropertyValue('--glass-fallback-scale').trim(),fallbackDistortion:Number(displacement?.getAttribute('scale')||0),horizontalOverflow:document.documentElement.scrollWidth>document.documentElement.clientWidth,verticalOverflow:document.documentElement.scrollHeight>document.documentElement.clientHeight}})()`)
  const clip = floatingClip.result?.value
  if(clip?.radius!=='20px'||!String(clip?.clipPath).includes('20px')||clip?.fallbackDistortion<=15||clip?.fallbackScale!=='1.045'||clip?.horizontalOverflow)throw new Error(`悬浮窗裁切或自定义折射异常：${JSON.stringify(clip)}`)
  console.log(JSON.stringify(clip))

  await evaluate(`(()=>{const config=JSON.parse(localStorage.getItem('token-manager-floating-config')||'{}');config.mode='capsule';config.miniMode=false;localStorage.setItem('token-manager-floating-config',JSON.stringify(config));location.reload()})()`)
  await sleep(1000)
  console.log(await capture('floating-capsule.png'))

  const collapseClick = await evaluate(`(()=>{const button=document.querySelector('button[title="折叠"]');button?.click();return Boolean(button)})()`)
  if(!collapseClick.result?.value)throw new Error('悬浮窗缺少折叠按钮。')
  await sleep(450)
  const collapsedProbe = await evaluate(`(()=>{const shell=document.querySelector('.floating-dashboard-shell');const header=document.querySelector('.floating-dashboard-header');const expand=document.querySelector('button[title="展开"]');const shellStyle=shell?getComputedStyle(shell):null;const headerStyle=header?getComputedStyle(header):null;const shellRect=shell?.getBoundingClientRect();const headerRect=header?.getBoundingClientRect();return{collapsed:shell?.classList.contains('collapsed'),headerDisplay:headerStyle?.display||'',headerWidth:headerRect?.width||0,headerHeight:headerRect?.height||0,expandVisible:Boolean(expand&&expand.getBoundingClientRect().width>0),expandLabel:expand?.getAttribute('aria-label')||'',radius:shellStyle?.borderRadius||'',clipPath:shellStyle?.clipPath||'',shellWidth:shellRect?.width||0,shellHeight:shellRect?.height||0,horizontalOverflow:document.documentElement.scrollWidth>document.documentElement.clientWidth}})()`)
  const collapsed = collapsedProbe.result?.value
  if(!collapsed?.collapsed||collapsed.headerDisplay==='none'||collapsed.headerWidth<1||collapsed.headerHeight<1||!collapsed.expandVisible||collapsed.radius!=='20px'||!String(collapsed.clipPath).includes('20px')||collapsed.horizontalOverflow)throw new Error(`悬浮窗收起状态异常：${JSON.stringify(collapsed)}`)
  console.log(JSON.stringify(collapsed))
  console.log(await capture('floating-window-white-collapsed.png'))
  await evaluate(`document.querySelector('button[title="展开"]')?.click()`)
  await sleep(450)
  const restoredProbe = await evaluate(`(()=>({collapsed:document.querySelector('.floating-dashboard-shell')?.classList.contains('collapsed'),collapseVisible:Boolean(document.querySelector('button[title="折叠"]')?.getBoundingClientRect().width)}))()`)
  const restored = restoredProbe.result?.value
  if(restored?.collapsed||!restored?.collapseVisible)throw new Error(`悬浮窗无法再次展开：${JSON.stringify(restored)}`)
  console.log(JSON.stringify(restored))
  console.log(await capture('floating-window-white-restored.png'))
} finally {
  socket.close()
  browser.kill()
  await new Promise(resolveClose => server.httpServer.close(resolveClose))
}
