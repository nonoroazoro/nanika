export function clampIndex(index: number, itemCount: number): number {
  if (itemCount <= 0) {
    return -1
  }
  return Math.min(Math.max(index, 0), itemCount - 1)
}
