import { expect, test, vi } from 'vitest'
import { userEvent } from 'vitest/browser'
import { render } from 'vitest-browser-svelte'

import { RootSearch } from '../../src/components'
import type { RootSearchSnapshot } from '../../src/types'

const snapshot: RootSearchSnapshot = {
  generation: 1,
  query: '',
  complete: true,
  results: [
    {
      extensionId: 'com.nanika.application',
      entryId: 'books',
      actionId: 'open',
      title: 'Books',
      subtitle: 'Application',
      iconUrl: null,
      kind: 'Application',
    },
    {
      extensionId: 'com.nanika.application',
      entryId: 'calendar',
      actionId: 'open',
      title: 'Calendar',
      subtitle: 'Application',
      iconUrl: null,
      kind: 'Application',
    },
  ],
}

test('keeps DOM focus in the search input while arrows move the active option', async () => {
  const screen = await render(RootSearch, {
    snapshot,
    onQuery: vi.fn(),
    onDismiss: vi.fn(),
    onInvoke: vi.fn(),
  })
  const input = screen.getByRole('combobox', { name: 'Search apps and commands' })

  await expect.element(input).toHaveFocus()
  await userEvent.keyboard('{ArrowDown}')
  await expect.element(input).toHaveFocus()
  await expect
    .element(screen.getByRole('option', { name: /Calendar/u }))
    .toHaveAttribute('aria-selected', 'true')
})

test('enter invokes the active result', async () => {
  const onInvoke = vi.fn()
  await render(RootSearch, {
    snapshot,
    onQuery: vi.fn(),
    onDismiss: vi.fn(),
    onInvoke,
  })

  await userEvent.keyboard('{Enter}')

  expect(onInvoke).toHaveBeenCalledWith(snapshot.results[0])
})
