import { writeFile } from 'node:fs/promises'

const [port = '9373', outputPath = '.tmp/native-floating-v085.png'] = process.argv.slice(2)
const sleep = ms => new Promise(resolve => setTimeout(resolve, ms))

async function targets() {
  return fetch(`http://127.0.0.1:${port}/json`).then(response => response.json())
}

async function connect(target) {
  const socket = new WebSocket(target.webSocketDebuggerUrl)
  await new Promise((resolve, reject) => {
    socket.addEventListener('open', resolve, { once: true })
    socket.addEventListener('error', reject, { once: true })
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
  const call = (method, params = {}) => new Promise((resolve, reject) => {
    const id = ++messageId
    pending.set(id, { resolve, reject })
    socket.send(JSON.stringify({ id, method, params }))
  })
  return { socket, call }
}

let discovered = []
for (let attempt = 0; attempt < 50; attempt++) {
  try {
    discovered = await targets()
    if (discovered.some(target => target.type === 'page')) break
  } catch {}
  await sleep(200)
}
let floatingTarget
for (let attempt = 0; attempt < 30; attempt++) {
  const list = await targets()
  for (const candidate of list.filter(target => target.type === 'page')) {
    const probe = await connect(candidate)
    try {
      const answer = await probe.call('Runtime.evaluate', {
        expression: `document.body.classList.contains('floating-window')`,
        returnByValue: true,
      })
      if (answer.result.value) {
        floatingTarget = candidate
        break
      }
    } finally {
      probe.socket.close()
    }
  }
  if (floatingTarget) break
  await sleep(200)
}
if (!floatingTarget) throw new Error('没有找到悬浮窗调试目标')

const floating = await connect(floatingTarget)
await floating.call('Page.enable')
await floating.call('Runtime.enable')
await floating.call('Runtime.evaluate', {
  expression: `localStorage.setItem('token-manager-floating-config',JSON.stringify({version:3,mode:'capsule',layout:'grid',enabled:['todayTokens','todayCalls'],order:['todayTokens','todayCalls'],mini:['todayTokens','todayCalls'],selectedKey:'__all__',visibleKeys:['__all__','__codex__','__claude__'],expandedKeys:[],interaction:'interactive',alwaysOnTop:true,snapToEdges:false,sizes:{capsule:{width:360,height:152},compact:{width:440,height:540},full:{width:560,height:760}},position:null}));location.reload()`,
})
await sleep(3200)
floating.socket.close()

// WebView2 在 reload 后会创建新的执行上下文，重新连接目标再读取真实窗口数据。
const refreshedTarget = (await targets()).find(target => target.id === floatingTarget.id) || floatingTarget
const refreshedFloating = await connect(refreshedTarget)
await refreshedFloating.call('Page.enable')
await refreshedFloating.call('Runtime.enable')
const stabilitySamples = []
for (let index = 0; index < 40; index++) {
  const sample = await refreshedFloating.call('Runtime.evaluate', {
    expression: `({screenX,screenY,innerWidth,innerHeight,mode:document.querySelector('.floating-shell')?.className||''})`,
    returnByValue: true,
  })
  stabilitySamples.push(sample.result.value)
  await sleep(100)
}
const distinctSizes = new Set(stabilitySamples.map(sample => `${sample.innerWidth}x${sample.innerHeight}`))
const movingSamples = stabilitySamples.slice(1).filter((sample,index) => sample.screenX !== stabilitySamples[index].screenX || sample.screenY !== stabilitySamples[index].screenY)
if (distinctSizes.size !== 1 || movingSamples.length) {
  throw new Error(`悬浮窗存在尺寸/位置自激抖动：${JSON.stringify({ distinctSizes:[...distinctSizes], movingSamples:movingSamples.length, samples:stabilitySamples.slice(0,8) })}`)
}
const before = await refreshedFloating.call('Runtime.evaluate', {
  expression: `(()=>{const root=document.querySelector('.floating-shell');const header=document.querySelector('.floating-header');const rect=root?.getBoundingClientRect();const drag=header?.getBoundingClientRect();return{className:root?.className,width:Math.round(rect?.width||0),height:Math.round(rect?.height||0),radius:root?getComputedStyle(root).borderRadius:'',screenX,screenY,dragX:(drag?.left||0)+Math.min(210,(drag?.width||300)*.58),dragY:(drag?.top||0)+(drag?.height||40)/2,nativeGlass:document.documentElement.dataset.nativeGlass||''}})()`,
  returnByValue: true,
})
const state = before.result.value
const modeChecks = []
for (const control of [
  { label: '展开悬浮窗', expected: 'mode-compact', size: '440x540' },
  { label: '切换为大版悬浮窗', expected: 'mode-full', size: '560x760' },
  { label: '折叠为胶囊', expected: 'mode-capsule', size: '360x152' },
]) {
  const clicked = await refreshedFloating.call('Runtime.evaluate', {
    expression: `(()=>{const button=document.querySelector('button[aria-label="${control.label}"]');if(!button)return false;button.click();return true})()`,
    returnByValue: true,
  })
  if (!clicked.result.value) throw new Error(`找不到悬浮窗模式按钮：${control.label}`)
  await sleep(650)
  const observed = await refreshedFloating.call('Runtime.evaluate', {
    expression: `({innerWidth,innerHeight,className:document.querySelector('.floating-shell')?.className||''})`,
    returnByValue: true,
  })
  const value = observed.result.value
  modeChecks.push({ label: control.label, ...value })
  if (!value.className.includes(control.expected) || `${value.innerWidth}x${value.innerHeight}` !== control.size) {
    throw new Error(`悬浮窗模式切换失败：${JSON.stringify({ control, observed:value })}`)
  }
}
await refreshedFloating.call('Input.dispatchMouseEvent', { type: 'mouseMoved', x: state.dragX, y: state.dragY })
await refreshedFloating.call('Input.dispatchMouseEvent', { type: 'mousePressed', x: state.dragX, y: state.dragY, button: 'left', buttons: 1, clickCount: 1 })
await sleep(120)
await refreshedFloating.call('Input.dispatchMouseEvent', { type: 'mouseMoved', x: state.dragX + 76, y: state.dragY + 48, button: 'left', buttons: 1 })
await sleep(180)
await refreshedFloating.call('Input.dispatchMouseEvent', { type: 'mouseReleased', x: state.dragX + 76, y: state.dragY + 48, button: 'left', buttons: 0, clickCount: 1 })
await sleep(350)
const after = await refreshedFloating.call('Runtime.evaluate', { expression: `({screenX,screenY})`, returnByValue: true })
const shot = await refreshedFloating.call('Page.captureScreenshot', { format: 'png', fromSurface: true, captureBeyondViewport: false })
await writeFile(outputPath, Buffer.from(shot.data, 'base64'))

const result = {
  ...state,
  stabilitySamples: stabilitySamples.length,
  stableSize: [...distinctSizes][0],
  spontaneousMoves: movingSamples.length,
  modeChecks,
  moved: state.screenX !== after.result.value.screenX || state.screenY !== after.result.value.screenY,
  afterX: after.result.value.screenX,
  afterY: after.result.value.screenY,
  outputPath,
}
if (result.width !== 360 || result.height !== 152 || Number.parseFloat(result.radius) < 28) {
  throw new Error(`原生悬浮窗验收失败：${JSON.stringify(result)}`)
}

refreshedFloating.socket.close()
process.stdout.write(JSON.stringify(result))
