import { spawn } from 'node:child_process'
import { existsSync } from 'node:fs'
import { mkdir, writeFile } from 'node:fs/promises'
import { join, resolve } from 'node:path'
import { preview } from 'vite'

const root = process.cwd()
const output = resolve(root, 'screenshots', 'settings-floating-nav')
const profile = resolve(root, '.tmp', `settings-floating-nav-${Date.now()}`)
await mkdir(output, { recursive: true })
await mkdir(profile, { recursive: true })

const edge = [
  join(process.env['PROGRAMFILES(X86)'] || '', 'Microsoft', 'Edge', 'Application', 'msedge.exe'),
  join(process.env.PROGRAMFILES || '', 'Microsoft', 'Edge', 'Application', 'msedge.exe'),
].find(existsSync)
if (!edge) throw new Error('找不到 Microsoft Edge')

const server = await preview({ root, configFile: false, preview: { host: '127.0.0.1', port: 4188, strictPort: true } })
const browser = spawn(edge, [
  '--headless=new', '--hide-scrollbars', '--remote-debugging-port=9348',
  `--user-data-dir=${profile}`, '--window-size=1440,900', 'http://127.0.0.1:4188/',
], { windowsHide: true, stdio: 'ignore' })

const sleep = ms => new Promise(resolveSleep => setTimeout(resolveSleep, ms))
let target
for (let attempt = 0; attempt < 40; attempt++) {
  try {
    target = (await fetch('http://127.0.0.1:9348/json').then(response => response.json())).find(item => item.type === 'page')
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
const runtimeErrors = []
const consoleEntries = []
socket.addEventListener('message', event => {
  const message = JSON.parse(String(event.data))
  if (message.method === 'Runtime.exceptionThrown') runtimeErrors.push(message.params?.exceptionDetails?.exception?.description || message.params?.exceptionDetails?.text || '未知异常')
  if (message.method === 'Runtime.consoleAPICalled') consoleEntries.push(message.params?.args?.map(item => item.value || item.description).join(' '))
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
const clickMainNav = label => evaluate(`(()=>{const button=[...document.querySelectorAll('aside > nav button')].find(item=>item.textContent?.trim()==='${label}');button?.click();return Boolean(button)})()`)

try {
  await call('Page.enable')
  await call('Runtime.enable')
  await sleep(800)
  await evaluate(`(()=>{[...document.querySelectorAll('button')].find(button=>button.textContent?.includes('稍后设置'))?.click();return true})()`)
  await sleep(250)

  for (const width of [1440, 980]) {
    await call('Emulation.setDeviceMetricsOverride', { width, height: width === 1440 ? 900 : 760, deviceScaleFactor: 1, mobile: false })
    await clickMainNav('设置')
    await sleep(350)
    const settingsAudit = (await evaluate(`(()=>{
      const center=document.querySelector('.settings-center')
      const nav=document.querySelector('.settings-nav')
      const detail=document.querySelector('.settings-detail')
      return {
        title:document.querySelector('.header-copy h1')?.textContent?.trim(),
        center:Boolean(center), single:center?.classList.contains('single-section'),
        categoryCount:nav?.querySelectorAll('button').length||0,
        navWidth:Math.round(nav?.getBoundingClientRect().width||0),
        detailWidth:Math.round(detail?.getBoundingClientRect().width||0),
        overflowX:document.documentElement.scrollWidth>innerWidth,
        pageHtml:document.querySelector('.page-panel')?.innerHTML?.slice(0,800),
        bodyText:document.body.innerText.slice(-600),
      }
    })()`)).result.value
    if (!settingsAudit.center || settingsAudit.single || settingsAudit.categoryCount < 6 || settingsAudit.overflowX) {
      throw new Error(`设置中心 ${width}px 验收失败：${JSON.stringify({ settingsAudit, runtimeErrors: runtimeErrors.slice(-3), consoleEntries: consoleEntries.slice(-8) })}`)
    }
    const settingsShot = await call('Page.captureScreenshot', { format: 'png', fromSurface: true, captureBeyondViewport: false })
    await writeFile(resolve(output, `settings-${width}.png`), Buffer.from(settingsShot.data, 'base64'))

    await clickMainNav('悬浮舱')
    await sleep(350)
    const floatingAudit = (await evaluate(`(()=>{
      const center=document.querySelector('.settings-center')
      const detail=document.querySelector('.settings-detail')
      return {
        title:document.querySelector('.header-copy h1')?.textContent?.trim(),
        center:Boolean(center), single:center?.classList.contains('single-section'),
        hasCategoryNav:Boolean(document.querySelector('.settings-nav')),
        hasEnableButton:[...document.querySelectorAll('.floating-control-action')].some(button=>button.textContent?.includes('开启')||button.textContent?.includes('关闭')),
        hasModePicker:document.querySelectorAll('.floating-shape-picker button').length===3,
        hasSizeEditor:Boolean(document.querySelector('.floating-size-editor')),
        detailWidth:Math.round(detail?.getBoundingClientRect().width||0),
        overflowX:document.documentElement.scrollWidth>innerWidth,
      }
    })()`)).result.value
    if (!floatingAudit.center || !floatingAudit.single || floatingAudit.hasCategoryNav || !floatingAudit.hasEnableButton || !floatingAudit.hasModePicker || !floatingAudit.hasSizeEditor || floatingAudit.overflowX) {
      throw new Error(`悬浮舱 ${width}px 验收失败：${JSON.stringify(floatingAudit)}`)
    }
    for (const mode of ['capsule', 'compact', 'full']) {
      const switched = (await evaluate(`(()=>{
        const order=['capsule','compact','full']
        const button=document.querySelectorAll('.floating-shape-picker button')[order.indexOf('${mode}')]
        button?.click()
        return Boolean(button)
      })()`)).result.value
      await sleep(120)
      const activeMode = (await evaluate(`(()=>{
        const active=[...document.querySelectorAll('.floating-shape-picker button')].findIndex(button=>button.classList.contains('active'))
        return ['capsule','compact','full'][active]||''
      })()`)).result.value
      if (!switched || activeMode !== mode) throw new Error(`悬浮舱 ${width}px 模式切换失败：目标 ${mode}，实际 ${activeMode}`)
    }
    const floatingShot = await call('Page.captureScreenshot', { format: 'png', fromSurface: true, captureBeyondViewport: false })
    await writeFile(resolve(output, `floating-${width}.png`), Buffer.from(floatingShot.data, 'base64'))
    console.log(JSON.stringify({ width, settingsAudit, floatingAudit }))
  }
} finally {
  socket.close()
  browser.kill()
  await new Promise(resolveClose => server.httpServer.close(resolveClose))
}
