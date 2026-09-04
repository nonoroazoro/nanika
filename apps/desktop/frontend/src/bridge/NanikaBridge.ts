import type { ApplicationSnapshot, InvokeCandidateRequest, RootSearchSnapshot } from '../types'

export interface NanikaBridge {
  openSession(listener: (snapshot: RootSearchSnapshot) => void): Promise<ApplicationSnapshot>
  publishQuery(query: string): Promise<RootSearchSnapshot>
  invokeCandidate(request: InvokeCandidateRequest): Promise<boolean>
  dismissLauncher(): Promise<void>
}
