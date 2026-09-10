const allowedCommands = new Set([
  'acknowledge_search',
  'close_session',
  'dismiss_launcher',
  'invoke_candidate',
  'open_session',
  'publish_query',
])

window.__TAURI_ISOLATION_HOOK__ = (message) => {
  // Tauri fetches large Channel payloads using this internal command with a null
  // body. Blocking it strands an ordered Channel behind its first large message.
  // Keep this exact exception; do not broadly allow plugin commands.
  if (message?.cmd === 'plugin:__TAURI_CHANNEL__|fetch' && message.payload === null) {
    return message
  }
  if (!message || typeof message !== 'object' || !allowedCommands.has(message.cmd)) {
    throw new Error('Blocked IPC command')
  }
  if (!message.payload || typeof message.payload !== 'object') {
    throw new Error('Blocked malformed IPC payload')
  }
  return message
}
