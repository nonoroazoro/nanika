import type { RootSearchSnapshot } from './index'

export interface ApplicationSnapshot {
  sessionId: number
  locale: string
  rootSearch: RootSearchSnapshot
}
