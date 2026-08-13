<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref } from 'vue'
import { getVersion } from '@tauri-apps/api/app'
import { isTauri } from '@tauri-apps/api/core'
import { openUrl } from '@tauri-apps/plugin-opener'
import { ArrowDownToLine, ChevronDown, X } from '@lucide/vue'
import { compareVersions, fetchReleaseManifest, releaseSummary, trackedDownloadUrl, type ReleaseManifestV2 } from '../features/releaseManifest'

const visible = ref(false)
const expanded = ref(false)
const currentVersion = ref('0.11.10')
const manifest = ref<ReleaseManifestV2 | null>(null)
const controller = new AbortController()
const summary = computed(() => releaseSummary(manifest.value))
const sizeText = computed(() => manifest.value?.size_bytes ? `${(manifest.value.size_bytes / 1_048_576).toFixed(1)} MB` : '大小待发布')

function canShow(next: ReleaseManifestV2) {
  if (compareVersions(next.version, currentVersion.value) <= 0) return false
  return Date.now() >= Number(localStorage.getItem(`token-manager-update-snooze-${next.version}`) || 0)
}
function acceptManifest(next: ReleaseManifestV2) {
  if (!canShow(next)) return
  manifest.value = next
  visible.value = true
}
function onRelease(event: Event) { acceptManifest((event as CustomEvent<ReleaseManifestV2>).detail) }
async function checkForNotice() {
  try {
    if (isTauri()) currentVersion.value = await getVersion()
    acceptManifest(await fetchReleaseManifest(controller.signal))
  } catch { /* 更新服务离线不影响本地监控。 */ }
}
function snooze() {
  if (manifest.value) localStorage.setItem(`token-manager-update-snooze-${manifest.value.version}`, String(Date.now() + 86_400_000))
  visible.value = false
}
async function downloadNow() { visible.value = false; await openUrl(trackedDownloadUrl(currentVersion.value)) }

onMounted(() => { window.addEventListener('token-manager-release-available', onRelease); void checkForNotice() })
onBeforeUnmount(() => { controller.abort(); window.removeEventListener('token-manager-release-available', onRelease) })
</script>

<template>
  <Transition name="update-notice">
    <aside v-if="visible && manifest" class="update-notice" role="dialog" aria-modal="false" aria-labelledby="update-notice-title">
      <header><span class="update-notice__icon"><ArrowDownToLine :size="18" /></span><span><small>稳定频道更新</small><b id="update-notice-title">Token Manager v{{ manifest.version }}</b></span><button type="button" aria-label="稍后提醒" title="稍后提醒" @click="snooze"><X :size="15" /></button></header>
      <p>{{ manifest.title || '发现新的稳定版本' }}</p>
      <div class="update-notice__meta"><span>{{ sizeText }}</span><span>{{ new Date(manifest.published_at).toLocaleDateString('zh-CN') }}</span></div>
      <Transition name="update-detail"><section v-if="expanded" class="update-notice__detail"><div v-if="summary.highlights.length"><b>新增功能</b><ul><li v-for="item in summary.highlights" :key="item">{{ item }}</li></ul></div><div v-if="summary.fixes.length"><b>修复问题</b><ul><li v-for="item in summary.fixes" :key="item">{{ item }}</li></ul></div><p v-if="!summary.highlights.length && !summary.fixes.length">{{ manifest.notes || '本次版本暂无详细说明。' }}</p></section></Transition>
      <footer><button class="update-notice__details" type="button" :aria-expanded="expanded" @click="expanded=!expanded">{{ expanded ? '收起说明' : '查看完整说明' }}<ChevronDown :size="14" :class="{ rotated: expanded }" /></button><button class="update-notice__snooze" type="button" @click="snooze">稍后提醒</button><button class="update-notice__download" type="button" @click="downloadNow">立即下载</button></footer>
    </aside>
  </Transition>
</template>

<style scoped>
.update-notice{position:fixed;z-index:12000;right:24px;bottom:24px;display:grid;width:min(410px,calc(100vw - 32px));gap:12px;padding:16px;border:1px solid color-mix(in srgb,var(--tm-ink,#1d1d1f) 11%,transparent);border-radius:16px;background:var(--tm-overlay-surface,#fff);box-shadow:0 22px 70px color-mix(in srgb,var(--tm-ink,#1d1d1f) 18%,transparent);color:var(--tm-overlay-ink,#1d1d1f);font-family:var(--apple-font,"PingFang SC","Microsoft YaHei",sans-serif)}.update-notice>header{display:grid;grid-template-columns:38px minmax(0,1fr) 30px;align-items:center;gap:10px}.update-notice__icon{display:grid;width:38px;height:38px;place-items:center;border-radius:12px;background:var(--tm-ink,#1d1d1f);color:var(--tm-on-ink,#fff)}.update-notice header>span:nth-child(2){display:grid;gap:2px}.update-notice small,.update-notice__meta{color:var(--tm-muted,#86868b);font-size:10px}.update-notice header b{font-size:14px}.update-notice header button{display:grid;width:30px;height:30px;place-items:center;padding:0;border:0;border-radius:10px;background:transparent;color:var(--tm-muted);cursor:pointer}.update-notice>p{margin:0;font-size:11px;line-height:1.55}.update-notice__meta{display:flex;gap:12px}.update-notice__detail{display:grid;gap:10px;padding:12px;border-radius:13px;background:var(--tm-surface,#f2f2f7);font-size:10px;line-height:1.6}.update-notice__detail>div{display:grid;gap:4px}.update-notice__detail ul{display:grid;gap:3px;margin:0;padding-left:18px}.update-notice__detail p{margin:0}.update-notice>footer{display:flex;align-items:center;justify-content:flex-end;gap:8px}.update-notice footer button{min-height:36px;padding:8px 12px;border:0;border-radius:12px;font:650 10px/1 var(--apple-font);cursor:pointer}.update-notice__details{display:flex;margin-right:auto;align-items:center;gap:5px;background:transparent;color:var(--tm-muted)}.update-notice__snooze{background:var(--tm-surface);color:var(--tm-ink)}.update-notice__download{background:var(--tm-ink,#1d1d1f);color:var(--tm-on-ink,#fff)}.update-notice__details svg{transition:transform var(--tm-motion-standard,220ms) var(--tm-ease-out)}.update-notice__details svg.rotated{transform:rotate(180deg)}.update-notice-enter-active,.update-notice-leave-active,.update-detail-enter-active,.update-detail-leave-active{transition:opacity var(--tm-motion-standard,220ms) var(--tm-ease-out),transform var(--tm-motion-standard,220ms) var(--tm-ease-out)}.update-notice-enter-from,.update-notice-leave-to{opacity:0;transform:translateY(1px) scale(.985)}.update-detail-enter-from,.update-detail-leave-to{opacity:0;transform:scale(.99)}@media(max-width:620px){.update-notice{right:16px;bottom:16px}}@media(prefers-reduced-motion:reduce){.update-notice-enter-active,.update-notice-leave-active,.update-detail-enter-active,.update-detail-leave-active,.update-notice__details svg{transition:none}}
</style>
