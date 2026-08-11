// 本地视觉验收辅助：通过 WebView2 的本机调试端口执行一条表达式。
const [, , endpoint, encodedExpression] = process.argv
if (!endpoint || !encodedExpression) {
  throw new Error('用法：node scripts/qa-cdp.mjs <webSocketDebuggerUrl> <expression>')
}
const expression = Buffer.from(encodedExpression, 'base64').toString('utf8')
if (process.env.TOKEN_MANAGER_QA_DEBUG === '1') process.stderr.write(`${JSON.stringify(expression)}\n`)
const socket = new WebSocket(endpoint)
const timeout = setTimeout(() => {
  socket.close()
  throw new Error('连接 WebView2 调试端口超时')
}, 8000)
socket.addEventListener('open', () => {
  socket.send(JSON.stringify({
    id: 1,
    method: 'Runtime.evaluate',
    params: { expression, awaitPromise: true, returnByValue: true },
  }))
})
socket.addEventListener('message', event => {
  const payload = JSON.parse(String(event.data))
  if (payload.id !== 1) return
  clearTimeout(timeout)
  process.stdout.write(JSON.stringify(payload.result))
  socket.close()
})
