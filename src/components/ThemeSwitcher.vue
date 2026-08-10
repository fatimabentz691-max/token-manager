<script setup lang="ts">
import { computed, ref } from 'vue'
import { appThemes, useThemePreferences } from '../features/themePreferences'

const open = ref(false)
const { theme, setTheme } = useThemePreferences()
const currentName = computed(() => theme.value.name)

function choose(id: string) {
  setTheme(id)
  open.value = false
}
</script>

<template>
  <div class="theme-switcher">
    <button class="theme-trigger" type="button" :aria-expanded="open" @click="open = !open">
      <i :style="{ background: 'var(--tm-accent)' }"></i>
      <span>{{ currentName }}</span>
      <b aria-hidden="true">⌄</b>
    </button>
    <Transition name="theme-pop">
      <div v-if="open" class="theme-menu" role="menu">
        <button v-for="item in appThemes" :key="item.id" type="button" role="menuitemradio" :aria-checked="theme.id === item.id" @click="choose(item.id)">
          <span class="theme-swatch" :style="{ background: item.background, color: item.ink }"><i :style="{ background: item.accent }"></i></span>
          <span><b>{{ item.name }}</b><small>{{ item.source }}</small></span>
          <em>{{ theme.id === item.id ? '✓' : '' }}</em>
        </button>
      </div>
    </Transition>
  </div>
</template>

<style scoped>
.theme-switcher{position:relative}.theme-trigger{display:flex;align-items:center;gap:8px;min-height:36px;padding:0 11px;border:1px solid var(--tm-line);border-radius:12px;background:var(--tm-glass);color:var(--tm-ink);font:inherit;font-size:10px}.theme-trigger i{width:9px;height:9px;border-radius:50%;box-shadow:0 0 0 4px color-mix(in srgb,var(--tm-accent) 14%,transparent)}.theme-trigger b{color:var(--tm-muted);font-size:10px}.theme-menu{position:absolute;z-index:40;top:43px;right:0;width:250px;display:grid;grid-template-columns:1fr 1fr;gap:6px;padding:9px;border:1px solid var(--tm-line);border-radius:18px;background:color-mix(in srgb,var(--tm-bg) 90%,transparent);box-shadow:var(--tm-shadow);backdrop-filter:blur(24px)}.theme-menu button{display:grid;grid-template-columns:28px 1fr 10px;align-items:center;gap:8px;padding:8px;border:0;border-radius:11px;background:transparent;color:var(--tm-ink);text-align:left}.theme-menu button:hover{background:var(--tm-surface-strong)}.theme-menu button>span:nth-child(2){display:grid;gap:2px}.theme-menu b{font-size:9px}.theme-menu small{color:var(--tm-muted);font-size:7px}.theme-menu em{font-style:normal}.theme-swatch{display:grid;place-items:center;width:28px;height:28px;border:1px solid currentColor;border-radius:9px}.theme-swatch i{width:9px;height:9px;border-radius:50%}.theme-pop-enter-active,.theme-pop-leave-active{transition:opacity .2s ease,transform .2s cubic-bezier(.22,1,.36,1)}.theme-pop-enter-from,.theme-pop-leave-to{opacity:0;transform:scale(.97) translateY(-3px)}
</style>
