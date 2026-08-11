<script setup lang="ts">
import { computed } from 'vue'
import type { UsageAccuracy, UsageSourceKind } from '../types'

const props = withDefaults(defineProps<{
  kind?: UsageSourceKind
  accuracy?: UsageAccuracy
  collectedAt?: string
  compact?: boolean
}>(), {
  kind: 'local_log',
  accuracy: 'observed',
  collectedAt: '',
  compact: false,
})

const labels: Record<UsageSourceKind, string> = {
  official_api: '官方接口',
  local_proxy: '代理实测',
  local_log: '本地日志',
  local_json: '本地 JSON',
  local_sqlite: '本地 SQLite',
  bill_import: '账单导入',
  estimate: '个人估算',
}

const accuracyLabels: Record<UsageAccuracy, string> = {
  official: '官方',
  observed: '实测',
  imported: '导入',
  estimated: '估算',
}

const age = computed(() => {
  if (!props.collectedAt) return ''
  const timestamp = Date.parse(props.collectedAt)
  if (!Number.isFinite(timestamp)) return ''
  const seconds = Math.max(0, Math.floor((Date.now() - timestamp) / 1000))
  if (seconds < 60) return `${seconds} 秒前`
  if (seconds < 3600) return `${Math.floor(seconds / 60)} 分钟前`
  return `${Math.floor(seconds / 3600)} 小时前`
})

const stale = computed(() => {
  if (!props.collectedAt) return false
  const timestamp = Date.parse(props.collectedAt)
  return Number.isFinite(timestamp) && Date.now() - timestamp > 10 * 60 * 1000
})
</script>

<template>
  <span class="source-badge" :class="[{ compact, stale }, `kind-${kind}`]" :title="age ? `采集于 ${age}` : '等待首次采集'">
    <i aria-hidden="true"></i>
    <b>{{ labels[kind] }}</b>
    <span v-if="!compact">{{ accuracyLabels[accuracy] }}</span>
    <small v-if="!compact && age">{{ age }}</small>
  </span>
</template>

<style scoped>
.source-badge{display:inline-flex;align-items:center;gap:6px;min-height:28px;padding:5px 10px;border:1px solid color-mix(in srgb,var(--tm-line) 82%,transparent);border-radius:999px;background:color-mix(in srgb,var(--tm-surface) 84%,transparent);color:var(--tm-muted);font-size:11px;line-height:1;white-space:nowrap}.source-badge i{width:7px;height:7px;border-radius:50%;background:var(--tm-ink)}.source-badge b{color:var(--tm-ink);font-weight:650}.source-badge small{padding-left:3px;color:var(--tm-muted)}.source-badge.compact{min-height:24px;padding:4px 8px}.source-badge.stale i{background:#ff9f0a}.source-badge.kind-estimate i{background:#ff9f0a}.source-badge.kind-official_api i{background:#34c759}.source-badge.kind-local_proxy i{background:#007aff}
</style>
