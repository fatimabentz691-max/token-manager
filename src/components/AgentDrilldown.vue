<script setup lang="ts">
import { computed, ref } from 'vue'
import type { Usage } from '../types'
import AnimatedNumber from './AnimatedNumber.vue'

const props = defineProps<{ usage: Usage[]; sourceAvailable?: boolean }>()
const tab = ref<'model' | 'project' | 'session'>('model')

function projectName(value?: string | null) {
  if (!value) return '未标注项目'
  const parts = value.replace(/\\/g, '/').split('/').filter(Boolean)
  return parts.length > 1 ? parts[parts.length - 2] : parts[0] || '未标注项目'
}
function sessionName(value?: string | null) {
  if (!value) return '未标注会话'
  const last = value.replace(/\\/g, '/').split('/').filter(Boolean).pop() || value
  return last.replace(/\.(jsonl?|db|sqlite)$/i, '').slice(0, 48)
}
const rows = computed(() => {
  const map = new Map<string, { key: string; token: number; cost: number; calls: number; latest: string; source: string }>()
  for (const item of props.usage) {
    const key = tab.value === 'model' ? item.model : tab.value === 'project' ? projectName(item.source_ref) : sessionName(item.source_ref)
    const current = map.get(key) || { key, token: 0, cost: 0, calls: 0, latest: item.at, source: item.source_kind || 'local_log' }
    current.token += item.input + item.output
    current.cost += item.cost
    current.calls += item.request_count || 1
    if (item.at > current.latest) current.latest = item.at
    map.set(key, current)
  }
  return [...map.values()].sort((a, b) => b.token - a.token).slice(0, 12)
})
</script>

<template>
  <section class="agent-drilldown card">
    <header><div><span>本地数据下钻</span><h2>模型、项目与会话</h2></div><span v-if="sourceAvailable===false" class="source-unavailable">来源当前不可用 · 历史记录已保留</span></header>
    <nav aria-label="本地 Agent 数据层级"><button v-for="item in [{id:'model',label:'模型'},{id:'project',label:'项目'},{id:'session',label:'会话'}]" :key="item.id" :class="{active:tab===item.id}" @click="tab=item.id as typeof tab">{{ item.label }}</button></nav>
    <div class="drilldown-list"><article v-for="row in rows" :key="row.key"><span><b>{{ row.key }}</b><small>{{ row.source }} · {{ new Date(row.latest).toLocaleString('zh-CN') }}</small></span><span><b><AnimatedNumber :value="row.token" format="compact" suffix=" Token" /></b><small>{{ row.calls }} 次 · ¥{{ row.cost.toFixed(4) }}</small></span></article><p v-if="!rows.length">尚无可下钻的真实记录；不会生成示例会话。</p></div>
  </section>
</template>

<style scoped>
.agent-drilldown{display:grid;gap:13px}.agent-drilldown header{display:flex;align-items:flex-start;justify-content:space-between;gap:16px}.agent-drilldown header>div{display:grid;gap:4px}.agent-drilldown header span{color:var(--tm-muted);font-size:9px}.agent-drilldown h2{margin:0;font-size:17px}.source-unavailable{padding:6px 9px;border-radius:99px;background:var(--tm-surface);color:var(--tm-ink)!important}.agent-drilldown nav{display:flex;gap:5px;padding:4px;border-radius:12px;background:var(--tm-surface)}.agent-drilldown nav button{flex:1;padding:8px;border:0;border-radius:9px;background:transparent;color:var(--tm-muted);font:inherit;font-size:10px}.agent-drilldown nav button.active{background:var(--tm-bg);color:var(--tm-ink);box-shadow:0 1px 3px rgba(0,0,0,.08)}.drilldown-list{display:grid;gap:1px;overflow:hidden;border:1px solid var(--tm-line);border-radius:14px;background:var(--tm-line)}.drilldown-list article{display:flex;align-items:center;justify-content:space-between;gap:18px;padding:11px 13px;background:var(--tm-bg)}.drilldown-list article>span{display:grid;gap:3px;min-width:0}.drilldown-list article>span:last-child{text-align:right}.drilldown-list b{overflow:hidden;font-size:11px;text-overflow:ellipsis;white-space:nowrap}.drilldown-list small{color:var(--tm-muted);font-size:9px}.drilldown-list p{margin:0;padding:20px;background:var(--tm-bg);color:var(--tm-muted);font-size:10px;text-align:center}@media(max-width:680px){.agent-drilldown header,.drilldown-list article{align-items:stretch;flex-direction:column}.drilldown-list article>span:last-child{text-align:left}}
</style>
