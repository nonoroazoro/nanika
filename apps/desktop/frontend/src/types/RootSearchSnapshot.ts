import type { SearchResult } from './index'

export interface RootSearchSnapshot {
  generation: number
  query: string
  results: SearchResult[]
  complete: boolean
}
