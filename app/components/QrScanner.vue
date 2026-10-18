<script setup lang="ts">
// Full-screen camera that reads one QR code. Runs in the webview
// (getUserMedia + jsQR), so it needs no Google Play Services.
import { X } from 'lucide-vue-next'
import jsQR from 'jsqr'

const emit = defineEmits<{ found: [text: string]; close: []; error: [message: string] }>()
const video = ref<HTMLVideoElement | null>(null)
let stream: MediaStream | null = null
let timer: ReturnType<typeof setTimeout> | undefined
const canvas = document.createElement('canvas')
const ctx = canvas.getContext('2d', { willReadFrequently: true })!

onMounted(async () => {
  try {
    stream = await navigator.mediaDevices.getUserMedia({
      video: { facingMode: 'environment', width: { ideal: 1280 }, height: { ideal: 720 } },
      audio: false,
    })
  } catch (e) {
    const denied = e instanceof DOMException && e.name === 'NotAllowedError'
    emit('error', denied ? 'Camera access is needed to scan. Allow it in Android settings, or enter the address instead.' : 'Couldn’t open the camera. Paste the link instead.')
    return
  }
  if (!video.value) return stop()
  video.value.srcObject = stream
  await video.value.play().catch(() => {})
  tick()
})
onUnmounted(stop)

function tick() {
  const v = video.value
  if (v && v.videoWidth) {
    // Read a centred square, scaled down: faster and enough for a phone screen.
    const side = Math.min(v.videoWidth, v.videoHeight)
    const size = Math.min(side, 640)
    canvas.width = canvas.height = size
    ctx.drawImage(v, (v.videoWidth - side) / 2, (v.videoHeight - side) / 2, side, side, 0, 0, size, size)
    const code = jsQR(ctx.getImageData(0, 0, size, size).data, size, size, { inversionAttempts: 'dontInvert' })
    if (code?.data) {
      haptic()
      stop()
      emit('found', code.data)
      return
    }
  }
  timer = setTimeout(tick, 120)
}

function stop() {
  clearTimeout(timer)
  stream?.getTracks().forEach((t) => t.stop())
  stream = null
}
</script>

<template>
  <div class="scanner" role="dialog" aria-label="Scan a pairing code">
    <video ref="video" playsinline muted />
    <div class="frame" />
    <p class="hint">Point at the QR code on the other device</p>
    <button class="icon-btn close" aria-label="Close" @click="stop(); emit('close')"><X :size="24" /></button>
  </div>
</template>

<style scoped>
.scanner { position: fixed; inset: 0; z-index: 50; background: #000; display: grid; place-items: center; }
video { position: absolute; inset: 0; width: 100%; height: 100%; object-fit: cover; }
.frame {
  position: relative; width: min(70vw, 70vh); aspect-ratio: 1; border-radius: 20px;
  box-shadow: 0 0 0 100vmax oklch(0 0 0 / 45%), inset 0 0 0 2px oklch(1 0 0 / 85%);
}
.hint { position: absolute; bottom: calc(env(safe-area-inset-bottom) + 48px); left: 16px; right: 16px; text-align: center; color: #fff; margin: 0; }
.close { position: absolute; top: calc(env(safe-area-inset-top) + 12px); right: 12px; color: #fff; }
</style>
