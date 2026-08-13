<script setup lang="ts">
import { computed, onMounted, ref } from 'vue'
import { getVersion } from '@tauri-apps/api/app'
import { invoke, isTauri } from '@tauri-apps/api/core'
import { openUrl } from '@tauri-apps/plugin-opener'
import { check, type Update } from '@tauri-apps/plugin-updater'
import { fetchReleaseManifest, releaseSummary, trackedDownloadUrl, type ReleaseManifestV2 } from '../features/releaseManifest'

defineProps<{ embedded?: boolean }>()

const currentVersion = ref('0.11.9')
const manifest = ref<ReleaseManifestV2 | null>(null)
const update = ref<Update | null>(null)
const checking = ref(false)
const installing = ref(false)
const downloaded = ref(0)
const total = ref(0)
const message = ref('启动后会检查稳定频道；你可以直接从官网下载，也可以在这里校验签名后安装。')
const progress = computed(() => total.value > 0 ? Math.min(100, Math.round(downloaded.value / total.value * 100)) : 0)
const fileSize = computed(() => manifest.value?.size_bytes ? `${(manifest.value.size_bytes / 1024 / 1024).toFixed(1)} MB` : '等待发布清单')
const summary = computed(() => releaseSummary(manifest.value))

async function loadManifest() {
  try { manifest.value = await fetchReleaseManifest() } catch { /* 离线不影响本地监控 */ }
}

async function checkForUpdate(silent = false) {
  if (checking.value || installing.value) return
  checking.value = true
  if (!silent) message.value = '正在检查稳定频道…'
  try {
    await loadManifest()
    if (!isTauri()) { message.value = '浏览器预览不会执行桌面更新检查。'; return }
    currentVersion.value = await getVersion()
    await update.value?.close().catch(() => undefined)
    update.value = await check({ timeout: 15_000 })
    message.value = update.value ? `发现 v${update.value.version}。可直接下载，或使用下方校验安装。` : '当前已经是稳定频道最新版本。'
  } catch (error) { message.value = `检查失败：${String(error)}` }
  finally { checking.value = false }
}

async function downloadFromOfficial() {
  try {
    await openUrl(trackedDownloadUrl(currentVersion.value))
    message.value = '已在系统浏览器打开官网下载；本次点击会经过下载统计入口。'
  } catch (error) { message.value = `无法打开下载地址：${String(error)}` }
}

async function installUpdate() {
  if (!update.value || installing.value) return
  if (!window.confirm(`准备校验并安装 Token Manager v${update.value.version}，是否继续？`)) return
  installing.value = true; downloaded.value = 0; total.value = 0
  message.value = '正在校验安装包大小与 SHA-256…'
  try {
    if (!manifest.value || manifest.value.version !== update.value.version) throw new Error('发布清单与更新频道版本不一致')
    await invoke('verify_update_artifact', { url: manifest.value.updater_url || manifest.value.download_url, expectedSha256: manifest.value.sha256, expectedSizeBytes: manifest.value.size_bytes })
    message.value = 'SHA-256 通过，正在下载并验证 Tauri 更新签名…'
    await update.value.downloadAndInstall(event => {
      if (event.event === 'Started') total.value = event.data.contentLength || 0
      if (event.event === 'Progress') downloaded.value += event.data.chunkLength
      if (event.event === 'Finished') message.value = '签名校验通过，正在启动安装程序…'
    }, { timeout: 10 * 60_000 })
  } catch (error) { message.value = `更新已中止：${String(error)}` }
  finally { installing.value = false }
}

onMounted(async () => {
  if (isTauri()) currentVersion.value = await getVersion().catch(() => currentVersion.value)
  await checkForUpdate(true)
})
</script>

<template>
  <section class="update-center" :class="{ embedded }" aria-labelledby="update-title">
    <header><div><span>软件更新</span><h2 id="update-title">安全稳定频道</h2><p>发布信息来自统一清单；直接下载会经过统计入口，应用内安装还会校验 SHA-256 与 Tauri 签名。</p></div><button :disabled="checking || installing" @click="checkForUpdate(false)">{{ checking ? '检查中…' : '检查更新' }}</button></header>
    <div class="update-version-row"><div><small>当前版本</small><b>v{{ currentVersion }}</b></div><div><small>最新版本</small><b>v{{ manifest?.version || update?.version || currentVersion }}</b></div><div><small>安装包大小</small><b>{{ fileSize }}</b></div><div><small>发布时间</small><b>{{ manifest?.published_at ? new Date(manifest.published_at).toLocaleString('zh-CN') : '—' }}</b></div></div>
    <div v-if="manifest && (summary.highlights.length || summary.fixes.length)" class="release-summary"><section v-if="summary.highlights.length"><b>新增功能</b><ul><li v-for="item in summary.highlights" :key="item">{{ item }}</li></ul></section><section v-if="summary.fixes.length"><b>修复问题</b><ul><li v-for="item in summary.fixes" :key="item">{{ item }}</li></ul></section></div>
    <dl v-if="manifest"><div><dt>频道</dt><dd>{{ manifest.channel || 'stable' }}</dd></div><div><dt>平台</dt><dd>{{ manifest.platform || 'windows-x86_64' }}</dd></div><div><dt>SHA-256</dt><dd>{{ manifest.sha256 || '等待发布' }}</dd></div></dl>
    <p class="update-message" role="status" aria-live="polite">{{ message }}</p>
    <div v-if="installing" class="update-progress"><i :style="{ width: `${progress}%` }" /><span>{{ progress }}%</span></div>
    <div class="update-actions"><button v-if="manifest" class="update-download" @click="downloadFromOfficial">立即下载</button><button v-if="update" class="update-install" :disabled="installing" @click="installUpdate">{{ installing ? '正在下载…' : '校验后安装' }}</button></div>
  </section>
</template>

<style scoped>
.update-center{display:grid;gap:16px;margin:18px 0;padding:22px;border:1px solid var(--tm-line);border-radius:18px;background:color-mix(in srgb,var(--tm-bg) 86%,transparent);color:var(--tm-ink)}.update-center.embedded{margin:0;padding:0;border:0;border-radius:0;background:transparent}.update-center>header{display:flex;align-items:flex-start;justify-content:space-between;gap:20px}.update-center>header div{display:grid;gap:5px;min-width:0}.update-center h2{margin:0;font-size:22px}.update-center header span,.update-center small,.update-center dt{color:var(--tm-muted);font-size:10px}.update-center header p{max-width:68ch;margin:0;color:var(--tm-muted);font-size:11px;line-height:1.6}.update-center button{flex:0 0 auto;padding:10px 14px;border:1px solid var(--tm-line);border-radius:12px;background:var(--tm-surface);color:var(--tm-ink);font:inherit;font-size:11px;font-weight:650}.update-center button:disabled{cursor:not-allowed;opacity:.5}.update-version-row{display:grid;grid-template-columns:repeat(4,minmax(0,1fr));gap:1px;overflow:hidden;border:1px solid var(--tm-line);border-radius:16px;background:var(--tm-line)}.update-version-row>div{display:grid;gap:5px;min-width:0;padding:15px;background:color-mix(in srgb,var(--tm-bg) 80%,transparent)}.update-version-row b{overflow-wrap:anywhere;font-size:13px;font-variant-numeric:tabular-nums}.release-summary{display:grid;grid-template-columns:repeat(2,minmax(0,1fr));gap:10px}.release-summary section{padding:14px;border:1px solid var(--tm-line);border-radius:14px;background:var(--tm-surface)}.release-summary b{font-size:12px}.release-summary ul{display:grid;gap:6px;margin:9px 0 0;padding-left:18px;color:var(--tm-muted);font-size:10px;line-height:1.5}.update-center dl{display:grid;gap:1px;margin:0;overflow:hidden;border:1px solid var(--tm-line);border-radius:14px;background:var(--tm-line)}.update-center dl>div{display:grid;grid-template-columns:90px minmax(0,1fr);gap:12px;padding:10px 12px;background:color-mix(in srgb,var(--tm-bg) 78%,transparent)}.update-center dd{min-width:0;margin:0;font:10px/1.5 ui-monospace,SFMono-Regular,Consolas,monospace;overflow-wrap:anywhere}.update-message{margin:0;color:var(--tm-muted);font-size:11px;line-height:1.55}.update-actions{display:flex;gap:8px;flex-wrap:wrap}.update-download{border-color:var(--tm-ink)!important;background:var(--tm-ink)!important;color:var(--tm-on-ink)!important}.update-install{background:var(--tm-surface)!important}.update-progress{position:relative;height:22px;overflow:hidden;border-radius:999px;background:var(--tm-surface)}.update-progress i{display:block;height:100%;border-radius:inherit;background:var(--tm-ink);transition:width var(--tm-motion-normal) var(--tm-ease-out)}.update-progress span{position:absolute;inset:0;display:grid;place-items:center;color:var(--tm-muted);font-size:9px;font-variant-numeric:tabular-nums}@media(max-width:900px){.update-version-row,.release-summary{grid-template-columns:repeat(2,minmax(0,1fr))}}@media(max-width:620px){.update-center>header{flex-direction:column}.update-center>header button{width:100%}.update-version-row,.release-summary{grid-template-columns:1fr}.update-center dl>div{grid-template-columns:1fr;gap:4px}}
</style>
