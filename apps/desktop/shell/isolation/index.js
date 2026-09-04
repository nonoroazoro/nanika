const allowedCommands = new Set([
  'dismiss_launcher',
  'invoke_candidate',
  'open_session',
  'publish_query',
])

window.__TAURI_ISOLATION_HOOK__ = (message) => {
  if (!message || typeof message !== 'object' || !allowedCommands.has(message.cmd)) {
    throw new Error('Blocked IPC command')
  }
  if (!message.payload || typeof message.payload !== 'object') {
    throw new Error('Blocked malformed IPC payload')
  }
  return message
}
