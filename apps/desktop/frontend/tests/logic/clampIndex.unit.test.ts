import { describe, expect, test } from 'vitest'

import { clampIndex } from '../../src/logic'

describe('clampIndex', () => {
  test('keeps selection inside a non-empty result set', () => {
    expect(clampIndex(-1, 3)).toBe(0)
    expect(clampIndex(1, 3)).toBe(1)
    expect(clampIndex(3, 3)).toBe(2)
  })

  test('uses no selection for an empty result set', () => {
    expect(clampIndex(0, 0)).toBe(-1)
  })
})
