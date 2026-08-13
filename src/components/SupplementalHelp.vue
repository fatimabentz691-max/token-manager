<script setup lang="ts">
import { Info } from '@lucide/vue'
import { computed, nextTick, onBeforeUnmount, ref, watch } from 'vue'
import { closeActiveSupplementalHelp, useDescriptionPreferences } from '../features/descriptionPreferences'

const props = withDefaults(defineProps<{
  helpId?: string
  summary?: string
  detail: string
  label?: string
  kind?: 'ordinary' | 'status' | 'source' | 'privacy' | 'empty' | 'error'
}>(), { helpId: '', summary: '', label: '', kind: 'ordinary' })
const { visibility, activeHelpId, setActiveHelp } = useDescriptionPreferences()
const root = ref<HTMLElement | null>(null)
const localId = `tm-help-${Math.random().toString(36).slice(2)}`
const id = computed(() => props.helpId || localId)
const locked = ref(false)
const hoverTimer = ref<number | null>(null)
const closeTimer = ref<number | null>(null)
const position = ref({ top: 0, left: 0, width: 320 })
const isError = computed(() => props.kind === 'error')
const open = computed(() => activeHelpId.value === id.value)
const inlineVisible = computed(() => visibility.value === 'always' || isError.value)
const interactive = computed(() => visibility.value === 'hover' && !isError.value)
const popoverVisible = computed(() => interactive.value && open.value)

function clearTimers() {
  if (hoverTimer.value !== null) window.clearTimeout(hoverTimer.value)
  if (closeTimer.value !== null) window.clearTimeout(closeTimer.value)
  hoverTimer.value = closeTimer.value = null
}

async function positionPopover() {
  await nextTick()
  const anchor = root.value?.getBoundingClientRect()
  if (!anchor) return
  const width = Math.min(340, Math.max(220, window.innerWidth - 24))
  const left = Math.max(12, Math.min(window.innerWidth - width - 12, anchor.left))
  const below = anchor.bottom + 8
  const top = below + 148 < window.innerHeight ? below : Math.max(12, anchor.top - 150)
  position.value = { top, left, width }
}

function show(delay = 90) {
  if (!interactive.value) return
  if (closeTimer.value !== null) window.clearTimeout(closeTimer.value)
  hoverTimer.value = window.setTimeout(() => {
    setActiveHelp(id.value)
    positionPopover()
  }, delay)
}
function hide() {
  if (locked.value) return
  if (hoverTimer.value !== null) window.clearTimeout(hoverTimer.value)
  closeTimer.value = window.setTimeout(() => {
    if (activeHelpId.value === id.value) setActiveHelp(null)
  }, 110)
}
function toggle() {
  if (!interactive.value) return
  clearTimers()
  locked.value = !open.value || !locked.value
  setActiveHelp(locked.value ? id.value : null)
  if (locked.value) positionPopover()
}
function close() { locked.value = false; if (open.value) setActiveHelp(null) }
function onDocumentPointer(event: PointerEvent) { if (locked.value && root.value && !root.value.contains(event.target as Node)) close() }
function onKeydown(event: KeyboardEvent) { if (event.key === 'Escape') close() }
watch(visibility, close)
window.addEventListener('keydown', onKeydown)
window.addEventListener('resize', positionPopover)
document.addEventListener('pointerdown', onDocumentPointer)
onBeforeUnmount(() => {
  clearTimers()
  if (open.value) closeActiveSupplementalHelp(id.value)
  window.removeEventListener('keydown', onKeydown)
  window.removeEventListener('resize', positionPopover)
  document.removeEventListener('pointerdown', onDocumentPointer)
})
</script>

<template>
  <span ref="root" class="supplemental-help" :class="[`visibility-${visibility}`, `kind-${kind}`, { open }]" @mouseenter="show()" @mouseleave="hide" @focusin="show(0)" @focusout="hide">
    <span class="supplemental-help__anchor"><slot /></span>
    <span v-if="inlineVisible && (summary || detail)" class="supplemental-summary">{{ summary || detail }}</span>
    <button v-if="interactive" type="button" :aria-label="label || '查看补充说明'" :aria-expanded="open" @click.stop="toggle"><Info :size="13" /></button>
    <Teleport to="body">
      <Transition name="help-pop">
        <span v-if="popoverVisible" class="supplemental-popover" :class="`kind-${kind}`" :style="{ top: `${position.top}px`, left: `${position.left}px`, width: `${position.width}px` }" role="tooltip">{{ detail }}</span>
      </Transition>
    </Teleport>
  </span>
</template>

<style scoped>
.supplemental-help{position:relative;display:inline-flex;max-width:100%;align-items:center;gap:6px;color:var(--tm-muted,#6e6e73)}.supplemental-help__anchor{display:inline-flex;min-width:0;align-items:center}.supplemental-summary{overflow:hidden;color:var(--tm-muted,#6e6e73);font-size:10px;line-height:1.55;text-overflow:ellipsis}.supplemental-help>button{display:grid;width:25px;height:25px;flex:0 0 auto;padding:0;place-items:center;border:1px solid var(--tm-line,#e5e5ea);border-radius:9px;background:color-mix(in srgb,var(--tm-bg,#fff) 90%,transparent);color:var(--tm-muted,#6e6e73);opacity:.62;transition:opacity var(--tm-motion-fast,160ms) ease,transform var(--tm-motion-standard,220ms) var(--tm-ease-out,cubic-bezier(.22,1,.36,1)),color var(--tm-motion-fast,160ms) ease}.supplemental-help:where(:hover,:focus-within) button,.supplemental-help.open>button{color:var(--tm-ink,#1d1d1f);opacity:1;transform:scale(1.025)}.supplemental-help>button:focus-visible{outline:2px solid color-mix(in srgb,var(--tm-accent,#007aff) 65%,transparent);outline-offset:2px}.supplemental-popover{position:fixed;z-index:20000;padding:11px 12px;border:1px solid color-mix(in srgb,var(--tm-ink,#1d1d1f) 13%,transparent);border-radius:13px;background:var(--tm-bg,#fff);box-shadow:0 16px 40px color-mix(in srgb,var(--tm-ink,#1d1d1f) 20%,transparent);color:var(--tm-ink,#1d1d1f);font-family:var(--apple-font,"PingFang SC","Microsoft YaHei",sans-serif);font-size:10px;line-height:1.65;white-space:normal}.supplemental-popover.kind-error{border-color:color-mix(in srgb,#ff3b30 34%,var(--tm-line))}.help-pop-enter-active,.help-pop-leave-active{transition:opacity var(--tm-motion-standard,220ms) var(--tm-ease-out,cubic-bezier(.22,1,.36,1)),transform var(--tm-motion-standard,220ms) var(--tm-ease-out,cubic-bezier(.22,1,.36,1))}.help-pop-enter-from,.help-pop-leave-to{opacity:0;transform:scale(.985)}@media(prefers-reduced-motion:reduce){.help-pop-enter-active,.help-pop-leave-active,.supplemental-help>button{transition:none}}:global(.motion-off) .help-pop-enter-active,:global(.motion-off) .help-pop-leave-active,:global(.motion-off) .supplemental-help>button{transition:none}
</style>
