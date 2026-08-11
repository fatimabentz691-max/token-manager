import { writeFile } from 'node:fs/promises'

const [webSocketUrl, outputPath] = process.argv.slice(2)
if (!webSocketUrl || !outputPath) {
  throw new Error('Usage: node qa-cdp-screenshot.mjs <websocket-url> <output-path>')
}

const socket = new WebSocket(webSocketUrl)
const pending = new Map()
let messageId = 0

function call(method, params = {}) {
  const id = ++messageId
  socket.send(JSON.stringify({ id, method, params }))
  return new Promise((resolve, reject) => pending.set(id, { resolve, reject }))
}

socket.addEventListener('message', (event) => {
  const message = JSON.parse(event.data)
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

await call('Page.enable')
const metrics = await call('Page.getLayoutMetrics')
const width = Math.ceil(metrics.cssLayoutViewport.clientWidth)
const height = Math.ceil(metrics.cssLayoutViewport.clientHeight)
const result = await call('Page.captureScreenshot', {
  format: 'png',
  captureBeyondViewport: false,
  fromSurface: true,
})

await writeFile(outputPath, Buffer.from(result.data, 'base64'))
socket.close()
process.stdout.write(JSON.stringify({ outputPath, width, height }))
