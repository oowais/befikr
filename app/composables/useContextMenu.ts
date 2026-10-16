// Context menus: right-click on desktop, long-press on Android (the webview
// fires `contextmenu` for a long-press too).
import type { Component } from 'vue'
import { reactive } from 'vue'
import { haptic } from './format'

export type MenuItem = { label: string; icon?: Component; run: () => unknown; danger?: boolean } | 'separator'

const state = reactive({ open: false, x: 0, y: 0, touch: false, openedAt: 0, items: [] as MenuItem[] })

export const useContextMenu = () => state

/** Use as `@contextmenu="openMenu($event, [...])"`. */
export function openMenu(e: MouseEvent, items: MenuItem[]) {
  if (!items.length) return
  e.preventDefault()
  e.stopPropagation()
  const touch = matchMedia('(pointer: coarse)').matches
  if (touch) haptic()
  Object.assign(state, { open: true, x: e.clientX, y: e.clientY, touch, openedAt: Date.now(), items })
}

export const closeMenu = () => (state.open = false)

/** Text fields and selected text keep the system menu (copy/paste);
 *  anywhere else the webview's own menu (Reload, Inspect…) is hidden. */
export function suppressDefaultMenu(e: MouseEvent) {
  if (e.defaultPrevented) return
  const t = e.target as HTMLElement | null
  if (t?.closest('input, textarea, [contenteditable="true"]')) return
  if (window.getSelection()?.toString()) return
  e.preventDefault()
}
