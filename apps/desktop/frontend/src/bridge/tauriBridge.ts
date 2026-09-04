import { Channel, invoke } from '@tauri-apps/api/core'

import type { RootSearchSnapshot } from '../types'
import type { NanikaBridge } from './index'

export const tauriBridge: NanikaBridge = {
  openSession: (listener) => {
    const updates = new Channel<RootSearchSnapshot>(listener)
    return invoke('open_session', { updates })
  },
  publishQuery: (query) => invoke('publish_query', { query }),
  invokeCandidate: (request) => invoke('invoke_candidate', { request }),
  dismissLauncher: () => invoke('dismiss_launcher'),
}
