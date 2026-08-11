<script setup lang="ts">
import { computed } from 'vue'
import ProviderMark from './ProviderMark.vue'
import AnimatedNumber from './AnimatedNumber.vue'
import type { Usage } from '../types'

const props = defineProps<{ usage: Usage[] }>()
const emit = defineEmits<{
  exportAll: []
  exportModel: [filter: { provider: string; model: string }]
}>()

const rows = computed(() => {
  const groups = new Map<string, { provider: string; model: string; calls: number; input: number; output: number; cached: number; cost: number; lastAt: string }>()
  for (const item of props.usage) {
    const key = `${item.provider}::${item.model}`
    const row = groups.get(key) || { provider: item.provider, model: item.model, calls: 0, input: 0, output: 0, cached: 0, cost: 0, lastAt: item.at }
    row.calls += 1
    row.input += item.input
    row.output += item.output
    row.cached += item.cached
    row.cost += item.cost
    if (new Date(item.at).getTime() > new Date(row.lastAt).getTime()) row.lastAt = item.at
    groups.set(key, row)
  }
  return [...groups.values()].sort((a, b) => b.cost - a.cost || (b.input + b.output) - (a.input + a.output))
})

const totalCost = computed(() => rows.value.reduce((sum, item) => sum + item.cost, 0))
const totalTokens = computed(() => rows.value.reduce((sum, item) => sum + item.input + item.output, 0))
const totalCalls = computed(() => rows.value.reduce((sum, item) => sum + item.calls, 0))
</script>

<template>
  <section class="report-center">
    <header class="report-heading">
      <div><span>本地账单中心</span><h2>按模型拆分累计消费</h2><p>每个模型独立汇总 Token、缓存、请求和人民币成本，可分别导出真正的 .xlsx 工作簿。</p></div>
      <button :disabled="!rows.length" @click="emit('exportAll')">导出全部模型 Excel</button>
    </header>
    <div class="report-summary">
      <article><span>累计 Token</span><strong><AnimatedNumber :value="totalTokens" /></strong></article>
      <article><span>累计消费</span><strong><AnimatedNumber :value="totalCost" :decimals="2" prefix="¥" /></strong></article>
      <article><span>API 请求</span><strong><AnimatedNumber :value="totalCalls" /></strong></article>
      <article><span>已记录模型</span><strong><AnimatedNumber :value="rows.length" /></strong></article>
    </div>
    <div v-if="rows.length" class="model-bills">
      <article v-for="row in rows" :key="`${row.provider}:${row.model}`" class="model-bill">
        <div class="model-identity"><ProviderMark :name="row.provider" /><div><b>{{ row.model }}</b><span>{{ row.provider }} · 最近 {{ new Date(row.lastAt).toLocaleString('zh-CN') }}</span></div></div>
        <div class="model-values">
          <span><small>累计 Token</small><b>{{ (row.input + row.output).toLocaleString() }}</b></span>
          <span><small>缓存命中</small><b>{{ row.cached.toLocaleString() }}</b></span>
          <span><small>请求次数</small><b>{{ row.calls.toLocaleString() }}</b></span>
          <span><small>累计消费</small><b>{{ row.cost ? `¥${row.cost.toFixed(4)}` : '未计价' }}</b></span>
        </div>
        <button class="export-one" @click="emit('exportModel', { provider: row.provider, model: row.model })">导出此模型 Excel</button>
      </article>
    </div>
    <div v-else class="report-empty"><b>还没有可生成账单的数据</b><span>开启本地 API 代理或同步账户后，模型会自动分类出现在这里。</span></div>
  </section>
</template>

<style scoped>
.report-center{display:grid;gap:16px}.report-heading{display:flex;align-items:flex-end;justify-content:space-between;gap:28px;padding:28px;border:1px solid #ededf0;border-radius:20px;background:#fff}.report-heading span,.model-identity span{color:#6e6e73;font-size:11px}.report-heading h2{margin:7px 0 6px;font-size:25px;letter-spacing:-.5px}.report-heading p{max-width:680px;margin:0;color:#6e6e73;font-size:12px;line-height:1.7}.report-heading button,.export-one{border:0;border-radius:12px;background:#111;color:#fff}.report-heading button{padding:12px 17px}.report-heading button:disabled{opacity:.35}.report-summary{display:grid;grid-template-columns:repeat(4,1fr);gap:12px}.report-summary article{display:grid;gap:9px;padding:20px;border-radius:16px;background:#f2f2f7}.report-summary span,.model-values small{color:#6e6e73;font-size:10px}.report-summary strong{font-size:22px}.model-bills{display:grid;gap:10px}.model-bill{display:grid;grid-template-columns:minmax(220px,1.2fr) minmax(420px,2fr) auto;align-items:center;gap:22px;padding:18px 20px;border:1px solid #ededf0;border-radius:16px;background:#fff}.model-identity{display:flex;align-items:center;gap:12px;min-width:0}.model-identity>div{display:grid;gap:5px;min-width:0}.model-identity b,.model-identity span{overflow:hidden;text-overflow:ellipsis;white-space:nowrap}.model-values{display:grid;grid-template-columns:repeat(4,1fr);gap:16px}.model-values span{display:grid;gap:5px}.model-values b{font-size:12px}.export-one{padding:9px 12px;font-size:10px}.report-empty{display:grid;gap:7px;padding:54px;text-align:center;border-radius:16px;background:#f5f5f7;color:#6e6e73}.report-empty b{color:#1d1d1f}@media(max-width:1050px){.model-bill{grid-template-columns:1fr}.report-summary{grid-template-columns:repeat(2,1fr)}}
</style>
<style scoped>
.report-heading,.model-bill{background:var(--tm-bg);border-color:var(--tm-line);color:var(--tm-ink)}.report-heading span,.report-heading p,.model-identity span,.model-values small{color:var(--tm-muted)}.report-heading button,.export-one{background:var(--tm-ink);color:var(--tm-on-ink)}.report-summary article,.report-empty{background:var(--tm-surface);color:var(--tm-ink)}.report-summary span,.report-empty span{color:var(--tm-muted)}
</style>
