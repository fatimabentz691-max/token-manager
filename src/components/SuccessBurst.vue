<script setup lang="ts">
import { ref, watch } from 'vue'
import { useMotionPreferences } from '../features/motionPreferences'
const props=defineProps<{trigger:number}>()
const {motionEnabled,particlesEnabled}=useMotionPreferences()
const visible=ref(false)
let timer=0
watch(()=>props.trigger,value=>{if(!value||!motionEnabled.value||!particlesEnabled.value||matchMedia('(prefers-reduced-motion: reduce)').matches)return;visible.value=false;requestAnimationFrame(()=>{visible.value=true});clearTimeout(timer);timer=window.setTimeout(()=>{visible.value=false},820)})
</script>

<template><span v-if="visible" class="success-burst" aria-hidden="true"><i v-for="index in 8" :key="index" :style="{'--particle':index}"></i></span></template>

<style scoped>
.success-burst{position:absolute;inset:50% auto auto 50%;z-index:3;pointer-events:none}.success-burst i{--angle:calc(var(--particle)*45deg);position:absolute;width:3px;height:3px;border-radius:50%;background:color-mix(in srgb,var(--tm-ink,#111) 28%,var(--tm-bg,#fff));animation:particle-out .8s cubic-bezier(.22,1,.36,1) both}@keyframes particle-out{0%{opacity:0;transform:rotate(var(--angle)) translateX(4px) scale(.4)}20%{opacity:.65}100%{opacity:0;transform:rotate(var(--angle)) translateX(30px)}}@media(prefers-reduced-motion:reduce){.success-burst{display:none}}:global(.motion-off) .success-burst{display:none}
</style>
