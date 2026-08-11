<script setup lang="ts">
import { computed } from 'vue'
import { AlertCircle, CheckCircle2, Database, RefreshCw, X } from '@lucide/vue'
import DataSourceBadge from './DataSourceBadge.vue'
import type { SyncHealth } from '../types'

const props = defineProps<{ open: boolean; items: SyncHealth[]; syncing: boolean }>()
const emit = defineEmits<{ close: []; refresh: [sourceId?: string] }>()

const healthy = computed(() => props.items.filter(item => item.status === 'healthy').length)
const errors = computed(() => props.items.filter(item => item.status === 'error').length)

function time(value: string | null) {
  if (!value) return '尚未成功同步'
  const date = new Date(value)
  return Number.isNaN(date.getTime()) ? '时间未知' : date.toLocaleString('zh-CN', { hour12: false })
}
</script>

<template>
  <Transition name="panel-fade">
    <div v-if="open" class="health-backdrop" @click.self="emit('close')">
      <aside class="health-panel" role="dialog" aria-modal="true" aria-label="监控健康中心">
        <header>
          <div>
            <span class="eyebrow">SYNC HEALTH</span>
            <h2>监控健康中心</h2>
            <p>{{ healthy }} 条正常 · {{ errors }} 条需要处理</p>
          </div>
          <button class="icon-button" aria-label="关闭" @click="emit('close')"><X :size="18" /></button>
        </header>

        <div v-if="!items.length" class="health-empty">
          <Database :size="24" />
          <b>还没有可监控的数据源</b>
          <span>前往“账户与模型”添加 API，或启用本地日志、SQLite、JSON 监控。</span>
        </div>

        <div v-else class="health-list">
          <article v-for="item in items" :key="item.source_id" class="health-card" :class="`status-${item.status}`">
            <div class="health-title">
              <component :is="item.status === 'error' ? AlertCircle : CheckCircle2" :size="18" />
              <div><b>{{ item.provider }}</b><small>{{ item.source_id }}</small></div>
              <DataSourceBadge :kind="item.source_kind" compact />
            </div>
            <p>{{ item.last_error || item.detail }}</p>
            <dl>
              <div><dt>最后成功</dt><dd>{{ time(item.last_success_at) }}</dd></div>
              <div><dt>同步延迟</dt><dd>{{ item.latency_ms }} ms</dd></div>
              <div><dt>本次新增</dt><dd>{{ item.imported }} 条</dd></div>
            </dl>
            <button class="refresh-source" :disabled="syncing || item.status === 'syncing'" @click="emit('refresh', item.source_id)">
              <RefreshCw :size="14" :class="{ spinning: item.status === 'syncing' }" />
              立即同步
            </button>
          </article>
        </div>

        <footer><button :disabled="syncing" @click="emit('refresh')"><RefreshCw :size="15" :class="{ spinning: syncing }" />同步全部数据源</button></footer>
      </aside>
    </div>
  </Transition>
</template>

<style scoped>
.health-backdrop{position:fixed;z-index:300;inset:0;display:flex;justify-content:flex-end;background:rgba(20,20,22,.14);backdrop-filter:blur(4px)}.health-panel{width:min(520px,calc(100vw - 24px));height:100%;overflow:auto;padding:26px;background:color-mix(in srgb,var(--tm-bg) 96%,transparent);border-left:1px solid var(--tm-line);box-shadow:-18px 0 50px rgba(0,0,0,.08)}header{display:flex;justify-content:space-between;gap:20px;margin-bottom:22px}h2{margin:4px 0 5px;font-size:28px;letter-spacing:-.04em}header p{margin:0;color:var(--tm-muted)}.eyebrow{font-size:10px;font-weight:750;letter-spacing:.18em}.icon-button{display:grid;width:38px;height:38px;padding:0;place-items:center;border-radius:12px}.health-list{display:grid;gap:12px}.health-card{display:grid;gap:14px;padding:17px;border:1px solid var(--tm-line);border-radius:16px;background:var(--tm-card)}.health-title{display:flex;align-items:center;gap:10px}.health-title>div{display:grid;min-width:0;margin-right:auto}.health-title small{overflow:hidden;color:var(--tm-muted);font-size:10px;text-overflow:ellipsis}.health-card p{min-height:32px;margin:0;color:var(--tm-muted);font-size:12px;line-height:1.55}.health-card dl{display:grid;grid-template-columns:repeat(3,1fr);gap:8px;margin:0}.health-card dl div{display:grid;gap:4px;padding:10px;border-radius:12px;background:var(--tm-surface)}dt{color:var(--tm-muted);font-size:10px}dd{margin:0;color:var(--tm-ink);font-size:11px;font-weight:650}.status-error{border-color:color-mix(in srgb,#ff3b30 34%,var(--tm-line))}.refresh-source{justify-self:start}.health-empty{display:grid;justify-items:center;gap:8px;padding:42px 24px;text-align:center;border:1px dashed var(--tm-line);border-radius:16px;color:var(--tm-muted)}.health-empty b{color:var(--tm-ink)}footer{position:sticky;bottom:-26px;margin:18px -26px -26px;padding:16px 26px 24px;background:linear-gradient(transparent,var(--tm-bg) 28%)}footer button{display:flex;width:100%;align-items:center;justify-content:center;gap:8px}.spinning{animation:spin .7s linear infinite}.panel-fade-enter-active,.panel-fade-leave-active{transition:opacity .2s ease}.panel-fade-enter-from,.panel-fade-leave-to{opacity:0}@keyframes spin{to{transform:rotate(360deg)}}@media(max-width:620px){.health-panel{padding:20px}.health-card dl{grid-template-columns:1fr}footer{margin:18px -20px -20px;padding:16px 20px 20px}}
</style>
