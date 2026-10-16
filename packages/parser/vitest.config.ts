import { defineConfig } from 'vitest/config'

// Default to UTC on every OS (Windows shells can't do `TZ=UTC cmd`).
// An explicit TZ wins, so CI can check other zones.
process.env.TZ ??= 'UTC'

export default defineConfig({})
