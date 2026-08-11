<script setup lang="ts">
import { onBeforeUnmount, ref, watch } from 'vue'
import { useMotionPreferences } from '../features/motionPreferences'

const props = withDefaults(defineProps<{ value:number; decimals?:number; prefix?:string; suffix?:string }>(), { decimals:0, prefix:'', suffix:'' })
const { motionEnabled } = useMotionPreferences()
const displayed = ref(0)
let frame = 0

function format(value:number) {
  return `${props.prefix}${new Intl.NumberFormat('zh-CN',{minimumFractionDigits:props.decimals,maximumFractionDigits:props.decimals}).format(value)}${props.suffix}`
}

watch(() => props.value, next => {
  cancelAnimationFrame(frame)
  if (!motionEnabled.value || matchMedia('(prefers-reduced-motion: reduce)').matches) { displayed.value=next; return }
  const from=displayed.value, started=performance.now()
  const tick=(time:number)=>{const progress=Math.min(1,(time-started)/300);displayed.value=from+(next-from)*(1-Math.pow(1-progress,4));if(progress<1)frame=requestAnimationFrame(tick)}
  frame=requestAnimationFrame(tick)
}, { immediate:true })
onBeforeUnmount(()=>cancelAnimationFrame(frame))
</script>

<template><span class="animated-number">{{format(displayed)}}</span></template>

<style scoped>
.animated-number{display:inline-block;font:inherit;font-variant-numeric:tabular-nums;transform-origin:left center;animation:number-settle .3s cubic-bezier(.2,.8,.2,1);transition:color .3s cubic-bezier(.22,1,.36,1)}@keyframes number-settle{from{opacity:.62;filter:blur(1.5px);transform:translateY(2px)}to{opacity:1;filter:none;transform:none}}
@media(prefers-reduced-motion:reduce){.animated-number{transition:none}}
:global(.motion-off) .animated-number{animation:none;transition:none}
</style>
