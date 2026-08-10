<script setup lang="ts">
import { chartCatalog, useChartPreferences, type ChartId, type DashboardMode } from '../features/chartPreferences'

withDefaults(defineProps<{ showMode?: boolean }>(), { showMode: true })

const { mode, enabled, setMode, setChartEnabled, resetCharts } = useChartPreferences()

function updateChart(id: ChartId, event: Event) {
  setChartEnabled(id, (event.target as HTMLInputElement).checked)
}
</script>

<template>
  <section class="chart-preferences" aria-labelledby="chart-preferences-title">
    <div class="preferences-heading">
      <div>
        <h2 id="chart-preferences-title">仪表盘显示模式</h2>
        <p>简单模式只保留四项核心信息；高级模式展示你勾选的全部分析图表。</p>
      </div>
      <div v-if="showMode" class="mode-switch" role="group" aria-label="仪表盘显示模式">
        <button :class="{ active: mode === 'simple' }" :aria-pressed="mode === 'simple'" @click="setMode('simple' as DashboardMode)">简单模式</button>
        <button :class="{ active: mode === 'advanced' }" :aria-pressed="mode === 'advanced'" @click="setMode('advanced' as DashboardMode)">高级模式</button>
      </div>
    </div>
    <div class="chart-toggle-list">
      <label v-for="item in chartCatalog" :key="item.id">
        <span><b>{{ item.title }}</b><small>{{ item.description }}</small></span>
        <input type="checkbox" :checked="enabled.includes(item.id)" @change="updateChart(item.id, $event)">
      </label>
    </div>
    <button class="reset-button" @click="resetCharts">恢复全部图表</button>
  </section>
</template>

<style scoped>
.chart-preferences{padding:0;background:#fff}.preferences-heading{display:flex;align-items:flex-start;justify-content:space-between;gap:24px}.preferences-heading h2{margin:0;font-size:17px;color:#1d1d1f}.preferences-heading p{max-width:64ch;margin:7px 0 0;color:#68686d;font-size:12px;line-height:1.55}.mode-switch{display:flex;gap:3px;padding:4px;border-radius:12px;background:#f2f2f7}.mode-switch button{padding:8px 13px;border:0;border-radius:9px;background:transparent;color:#525257;font-size:12px}.mode-switch button.active{background:#1d1d1f;color:#fff}.chart-toggle-list{display:grid;grid-template-columns:repeat(2,minmax(0,1fr));gap:1px;margin-top:20px;overflow:hidden;border-radius:12px;background:#e5e5ea}.chart-toggle-list label{display:flex;align-items:center;justify-content:space-between;gap:16px;padding:14px 16px;background:#fff;cursor:pointer}.chart-toggle-list span,.chart-toggle-list b,.chart-toggle-list small{display:block}.chart-toggle-list b{font-size:12px;color:#1d1d1f}.chart-toggle-list small{margin-top:4px;color:#68686d;font-size:10px}.chart-toggle-list input{width:18px;height:18px;margin:0;accent-color:#111}.reset-button{margin-top:16px;padding:8px 12px;border:1px solid #d8d8dc;border-radius:10px;background:#fff;color:#1d1d1f;font-size:11px}@media(max-width:900px){.preferences-heading{flex-direction:column}.chart-toggle-list{grid-template-columns:1fr}}
</style>
<style scoped>
.chart-preferences,.chart-toggle-list label{background:var(--tm-bg);color:var(--tm-ink)}.preferences-heading h2,.chart-toggle-list b{color:var(--tm-ink)}.preferences-heading p,.chart-toggle-list small{color:var(--tm-muted)}.chart-toggle-list{background:var(--tm-line)}.mode-switch{background:var(--tm-surface)}.mode-switch button{color:var(--tm-muted)}.mode-switch button.active{background:var(--tm-ink);color:var(--tm-on-ink)}.reset-button{border-color:var(--tm-line);background:var(--tm-bg);color:var(--tm-ink)}.chart-toggle-list input{accent-color:var(--tm-ink)}
</style>
