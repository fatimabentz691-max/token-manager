// 原生 WebView2 悬浮窗拖动验收：从标题栏空白处拖动，并返回窗口移动前后坐标。
const [webSocketUrl] = process.argv.slice(2)
if (!webSocketUrl) throw new Error('Usage: node qa-cdp-drag.mjs <websocket-url>')

const socket = new WebSocket(webSocketUrl)
const pending = new Map()
let messageId = 0
const sleep = ms => new Promise(resolve => setTimeout(resolve, ms))

function call(method, params = {}) {
  const id = ++messageId
  socket.send(JSON.stringify({ id, method, params }))
  return new Promise((resolve, reject) => pending.set(id, { resolve, reject }))
}

socket.addEventListener('message', event => {
  const message = JSON.parse(String(event.data))
  if (!message.id || !pending.has(message.id)) return
  const request = pending.get(message.id)
  pending.delete(message.id)
  if (message.error) request.reject(new Error(message.error.message))
  else request.resolve(message.result)
})

await new Promise((resolve, reject) => {
  socket.addEventListener('open', resolve, { once: true })
  socket.addEventListener('error', reject, { once: true })
})

const before = await call('Runtime.evaluate', {
  expression: `(()=>{const rect=document.querySelector('.floating-classic__header')?.getBoundingClientRect();return{screenX,screenY,x:(rect?.left||0)+Math.min(210,(rect?.width||300)*.58),y:(rect?.top||0)+(rect?.height||40)/2}})()`,
  returnByValue: true,
})
const start = before.result.value
await call('Input.dispatchMouseEvent', { type: 'mouseMoved', x: start.x, y: start.y })
await call('Input.dispatchMouseEvent', { type: 'mousePressed', x: start.x, y: start.y, button: 'left', buttons: 1, clickCount: 1 })
await sleep(120)
await call('Input.dispatchMouseEvent', { type: 'mouseMoved', x: start.x + 72, y: start.y + 46, button: 'left', buttons: 1 })
await sleep(180)
await call('Input.dispatchMouseEvent', { type: 'mouseReleased', x: start.x + 72, y: start.y + 46, button: 'left', buttons: 0, clickCount: 1 })
await sleep(260)
const after = await call('Runtime.evaluate', { expression: `({screenX,screenY})`, returnByValue: true })
socket.close()

const result = {
  before: { x: start.screenX, y: start.screenY },
  after: { x: after.result.value.screenX, y: after.result.value.screenY },
}
result.moved = result.before.x !== result.after.x || result.before.y !== result.after.y
process.stdout.write(JSON.stringify(result))
