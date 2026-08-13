<script setup lang="ts">
import { computed, onBeforeUnmount, ref, watch } from 'vue'
import { useMotionPreferences } from '../features/motionPreferences'

export type AnimatedNumberFormat = 'integer' | 'compact' | 'currency' | 'percent'

const props = withDefaults(defineProps<{
  value: number
  decimals?: number
  prefix?: string
  suffix?: string
  format?: AnimatedNumberFormat
}>(), { decimals: 0, prefix: '', suffix: '', format: 'integer' })
const { motionEnabled } = useMotionPreferences()
const displayed = ref(0)
const changed = ref(false)
const widthCh = ref(1)
let frame = 0
let changeTimer = 0

function formatValue(value: number) {
  const safe = Number.isFinite(value) ? value : 0
  let text = ''
  if (props.format === 'compact') {
    text = new Intl.NumberFormat('zh-CN', {
      notation: 'compact',
      compactDisplay: 'short',
      minimumFractionDigits: props.decimals,
      maximumFractionDigits: Math.max(props.decimals, 1),
    }).format(safe)
  } else if (props.format === 'currency') {
    text = new Intl.NumberFormat('zh-CN', {
      style: 'currency',
      currency: 'CNY',
      minimumFractionDigits: props.decimals,
      maximumFractionDigits: props.decimals,
    }).format(safe)
  } else if (props.format === 'percent') {
    text = `${new Intl.NumberFormat('zh-CN', {
      minimumFractionDigits: props.decimals,
      maximumFractionDigits: props.decimals,
    }).format(safe)}%`
  } else {
    text = new Intl.NumberFormat('zh-CN', {
      minimumFractionDigits: props.decimals,
      maximumFractionDigits: props.decimals,
    }).format(safe)
  }
  return `${props.prefix}${text}${props.suffix}`
}

const displayedText = computed(() => formatValue(displayed.value))

watch(() => props.value, next => {
  cancelAnimationFrame(frame)
  window.clearTimeout(changeTimer)
  widthCh.value = Math.max(widthCh.value, formatValue(displayed.value).length, formatValue(next).length)
  changed.value = next !== displayed.value
  changeTimer = window.setTimeout(() => { changed.value = false; widthCh.value = formatValue(next).length }, 360)
  if (!motionEnabled.value || matchMedia('(prefers-reduced-motion: reduce)').matches) { displayed.value=next; return }
  const from=displayed.value, started=performance.now()
  const tick=(time:number)=>{const progress=Math.min(1,(time-started)/320);displayed.value=from+(next-from)*(1-Math.pow(1-progress,4));if(progress<1)frame=requestAnimationFrame(tick);else displayed.value=next}
  frame=requestAnimationFrame(tick)
}, { immediate:true })
onBeforeUnmount(()=>{cancelAnimationFrame(frame);window.clearTimeout(changeTimer)})
</script>

<template><span class="animated-number" :class="{ changed }" :style="{ '--number-width': `${widthCh}ch` }">{{ displayedText }}</span></template>

<style scoped>
.animated-number{display:inline-block;min-width:var(--number-width,1ch);font:inherit;font-variant-numeric:tabular-nums;text-align:inherit;transform-origin:left center;transition:min-width var(--tm-motion-data,300ms) var(--tm-ease-out,cubic-bezier(.22,1,.36,1)),color var(--tm-motion-data,300ms) var(--tm-ease-out,cubic-bezier(.22,1,.36,1)),filter var(--tm-motion-data,300ms) var(--tm-ease-out,cubic-bezier(.22,1,.36,1))}.animated-number.changed{color:var(--tm-accent,#007aff);filter:brightness(1.08)}
@media(prefers-reduced-motion:reduce){.animated-number{transition:none}}
:global(.motion-off) .animated-number{animation:none;transition:none}
</style>
