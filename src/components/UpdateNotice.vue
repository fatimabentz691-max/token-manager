<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref } from 'vue'
import { getVersion } from '@tauri-apps/api/app'
import { isTauri } from '@tauri-apps/api/core'
import { openUrl } from '@tauri-apps/plugin-opener'
import { ChevronDown, X } from '@lucide/vue'
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
    <section v-if="visible && manifest" class="update-notice" role="status" aria-live="polite" aria-labelledby="update-notice-title">
      <div class="update-notice-badge">更新</div>
      <div class="update-notice-copy">
        <b id="update-notice-title">Token Manager v{{ manifest.version }} · {{ manifest.title || '发现新的稳定版本' }}</b>
        <p><span>{{ sizeText }}</span><span>发布于 {{ new Date(manifest.published_at).toLocaleDateString('zh-CN') }}</span></p>
      </div>
      <button v-if="summary.highlights.length || summary.fixes.length || manifest.notes" class="update-notice-details" type="button" :aria-expanded="expanded" @click="expanded=!expanded">{{ expanded ? '收起说明' : '查看完整说明' }}<ChevronDown :size="14" :class="{ rotated: expanded }" /></button>
      <button class="update-notice-download" type="button" @click="downloadNow">立即下载</button>
      <button class="update-notice-close" type="button" aria-label="稍后提醒" title="稍后提醒" @click="snooze"><X :size="15" /></button>
      <Transition name="update-detail"><section v-if="expanded" class="update-notice-detail"><div v-if="summary.highlights.length"><b>新增功能</b><ul><li v-for="item in summary.highlights" :key="item">{{ item }}</li></ul></div><div v-if="summary.fixes.length"><b>修复问题</b><ul><li v-for="item in summary.fixes" :key="item">{{ item }}</li></ul></div><p v-if="!summary.highlights.length && !summary.fixes.length">{{ manifest.notes || '本次版本暂无详细说明。' }}</p></section></Transition>
    </section>
  </Transition>
</template>

<style scoped>
/* 更新公告与“软件公告”使用同一位置与视觉语言：内容区顶部的内嵌横幅，
 * 不使用固定定位，因此不会被左侧菜单栏或任何主题材质规则遮挡。 */
.update-notice{display:grid;grid-template-columns:auto minmax(0,1fr) auto auto auto;align-items:center;gap:12px;margin:-8px 0 18px;padding:14px 15px;border:1px solid var(--tm-line);border-radius:16px;background:var(--tm-bg);color:var(--tm-ink);font-family:var(--apple-font,"PingFang SC","Microsoft YaHei",sans-serif)}
.update-notice-badge{padding:5px 8px;border-radius:99px;background:var(--tm-ink);color:var(--tm-on-ink);font-size:9px;font-weight:700;letter-spacing:.06em}
.update-notice-copy{display:grid;min-width:0;gap:4px}
.update-notice-copy b{overflow:hidden;font-size:12px;text-overflow:ellipsis;white-space:nowrap}
.update-notice-copy p{display:flex;flex-wrap:wrap;gap:10px;margin:0;color:var(--tm-muted);font-size:10px;line-height:1.55}
.update-notice-details{display:flex;align-items:center;gap:5px;padding:8px 12px!important;border:1px solid var(--tm-line)!important;border-radius:10px!important;background:var(--tm-surface)!important;color:var(--tm-muted)!important;font:inherit;font-size:10px;cursor:pointer;white-space:nowrap}
.update-notice-details:hover{color:var(--tm-ink)!important}
.update-notice-details svg{transition:transform var(--tm-motion-standard,220ms) var(--tm-ease-out)}
.update-notice-details svg.rotated{transform:rotate(180deg)}
.update-notice-download{padding:8px 12px!important;border-radius:10px!important;background:var(--tm-ink)!important;color:var(--tm-on-ink)!important;font:inherit;font-size:10px;font-weight:650;white-space:nowrap}
.update-notice-close{display:grid;width:30px;height:30px;place-items:center;padding:0!important;border-radius:9px!important;background:transparent!important;color:var(--tm-muted)!important;font-size:15px}
.update-notice-close:hover{background:var(--tm-surface)!important;color:var(--tm-ink)!important}
.update-notice-detail{display:grid;grid-column:1/-1;gap:10px;padding:12px;border-radius:13px;background:var(--tm-surface);font-size:10px;line-height:1.6}
.update-notice-detail>div{display:grid;gap:4px}
.update-notice-detail ul{display:grid;gap:3px;margin:0;padding-left:18px;color:var(--tm-muted)}
.update-notice-detail p{margin:0;color:var(--tm-muted)}
.update-notice-enter-active,.update-notice-leave-active,.update-detail-enter-active,.update-detail-leave-active{transition:opacity var(--tm-motion-standard,220ms) var(--tm-ease-out),transform var(--tm-motion-standard,220ms) var(--tm-ease-out)}
.update-notice-enter-from,.update-notice-leave-to{opacity:0;transform:translateY(-4px)}
.update-detail-enter-from,.update-detail-leave-to{opacity:0;transform:scale(.99)}
@media(max-width:760px){.update-notice{grid-template-columns:auto minmax(0,1fr) auto auto}.update-notice-copy{grid-column:2/5}.update-notice-details{grid-column:2}.update-notice-download{grid-column:3}.update-notice-close{grid-column:4;grid-row:1}.update-notice-detail{grid-column:1/-1}}
@media(prefers-reduced-motion:reduce){.update-notice-enter-active,.update-notice-leave-active,.update-detail-enter-active,.update-detail-leave-active,.update-notice-details svg{transition:none}}
</style>
