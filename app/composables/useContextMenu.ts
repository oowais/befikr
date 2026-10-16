// Right-click menus on desktop (Linux, Windows). Android keeps its own
// long-press behaviour, so nothing opens there.
import type { Component } from 'vue'
import { reactive } from 'vue'

export type MenuItem = { label: string; icon?: Component; run: () => unknown; danger?: boolean } | 'separator'

const state = reactive({ open: false, x: 0, y: 0, items: [] as MenuItem[] })

export const isDesktopUi = () => typeof navigator !== 'undefined' && !/Android/i.test(navigator.userAgent)

export const useContextMenu = () => state

/** Use as `@contextmenu="openMenu($event, [...])"`. */
export function openMenu(e: MouseEvent, items: MenuItem[]) {
  if (!isDesktopUi() || !items.length) return
  e.preventDefault()
  e.stopPropagation()
  Object.assign(state, { open: true, x: e.clientX, y: e.clientY, items })
}

export const closeMenu = () => (state.open = false)

/** Text fields and selected text keep the system menu (copy/paste); anywhere
 *  else the webview's own menu (Reload, Inspect…) is hidden. */
export function suppressDefaultMenu(e: MouseEvent) {
  if (!isDesktopUi() || e.defaultPrevented) return
  const t = e.target as HTMLElement | null
  if (t?.closest('input, textarea, [contenteditable="true"]')) return
  if (window.getSelection()?.toString()) return
  e.preventDefault()
}
