<script setup lang="ts">
// The app icon, drawn inline so it follows the theme.
</script>

<template>
  <svg viewBox="0 0 108 108" class="logo" aria-hidden="true">
    <rect width="108" height="108" rx="26" class="logo-bg" />
    <g transform="translate(54 54) scale(0.95) translate(-54 -54)" fill="none" stroke-linecap="round" stroke-linejoin="round">
      <path d="M26 38 l3.6 3.6 l7.2 -8 M43 38 q9 -7 18 0 t19 0" class="l1" stroke-width="5.5" />
      <path d="M26 54 l3.6 3.6 l7.2 -8 M43 54 q10 -8 21 0 t22 0" class="l2" stroke-width="6.5" />
      <circle cx="31.5" cy="70" r="4.8" class="l3" stroke-width="3.6" />
      <path d="M43 70 q8 -7 16.5 0 t16.5 0" class="l1" stroke-width="5.5" />
    </g>
  </svg>
</template>

<style scoped>
.logo { width: 88px; height: 88px; filter: drop-shadow(0 8px 18px var(--glass-shade)); }
.logo-bg { fill: var(--logo-bg); }
.l1 { stroke: var(--logo-1); }
.l2 { stroke: var(--logo-2); }
.l3 { stroke: var(--logo-3); }
</style>

<style>
/* Unscoped: a scoped `:global(:root) .x` compiles to just `:root`. */
:root { --logo-bg: #e3ecdc; --logo-1: #7a9a76; --logo-2: #24452a; --logo-3: #e07a5f; }
@media (prefers-color-scheme: dark) {
  :root:not([data-theme="light"]) { --logo-bg: #23262b; --logo-1: #b9f6ca; --logo-2: #ffffff; --logo-3: #69f0ae; }
}
:root[data-theme="dark"] { --logo-bg: #23262b; --logo-1: #b9f6ca; --logo-2: #ffffff; --logo-3: #69f0ae; }
</style>
