<script setup lang="ts">
import { computed, onMounted, ref } from 'vue'
import { getVersion } from '@tauri-apps/api/app'
import { invoke, isTauri } from '@tauri-apps/api/core'
import { check, type Update } from '@tauri-apps/plugin-updater'
import { currentCloudBaseUrl } from '../features/cloudConfig'

interface ReleaseManifestV2 {
  version: string
  channel: string
  platform: string
  size_bytes: number
  sha256: string
  download_url: string
  updater_url?: string
  notes: string
  published_at: string
}

const currentVersion = ref('0.11.0')
const manifest = ref<ReleaseManifestV2 | null>(null)
const update = ref<Update | null>(null)
const checking = ref(false)
const installing = ref(false)
const downloaded = ref(0)
const total = ref(0)
const message = ref('启动时会自动检查稳定频道；是否安装始终由你决定。')

const progress = computed(() => total.value > 0 ? Math.min(100, Math.round(downloaded.value / total.value * 100)) : 0)
const fileSize = computed(() => {
  const size = manifest.value?.size_bytes || 0
  return size ? `${(size / 1024 / 1024).toFixed(1)} MB` : '等待发布清单'
})

async function loadManifest() {
  try {
    const response = await fetch(`${currentCloudBaseUrl()}/v1/release/latest`, { cache: 'no-store' })
    if (response.ok) manifest.value = await response.json() as ReleaseManifestV2
  } catch {
    // 更新器仍会独立校验签名；这里仅用于显示版本、大小和哈希。
  }
}

async function checkForUpdate(silent = false) {
  if (checking.value || installing.value) return
  checking.value = true
  if (!silent) message.value = '正在检查稳定频道…'
  try {
    await loadManifest()
    if (!isTauri()) {
      message.value = '浏览器预览不执行桌面更新检查。'
      return
    }
    currentVersion.value = await getVersion()
    update.value?.close().catch(() => undefined)
    update.value = await check({ timeout: 15_000 })
    message.value = update.value
      ? `发现 v${update.value.version}，下载前会再次校验更新签名。`
      : '当前已经是稳定频道最新版本。'
  } catch (error) {
    message.value = `检查失败：${String(error)}`
  } finally {
    checking.value = false
  }
}

async function installUpdate() {
  if (!update.value || installing.value) return
  if (!window.confirm(`准备下载 Token Manager v${update.value.version}。下载完成后将提示关闭程序并安装，是否继续？`)) return
  installing.value = true
  downloaded.value = 0
  total.value = 0
  message.value = '正在安全下载更新包…'
  try {
    if (!manifest.value || manifest.value.version !== update.value.version) {
      throw new Error('统一发布清单与更新频道版本不一致')
    }
    message.value = '正在复核安装包大小与 SHA-256…'
    await invoke('verify_update_artifact', {
      url: manifest.value.updater_url || manifest.value.download_url,
      expectedSha256: manifest.value.sha256,
      expectedSizeBytes: manifest.value.size_bytes,
    })
    message.value = 'SHA-256 通过，正在下载并验证 Tauri 更新签名…'
    await update.value.downloadAndInstall(event => {
      if (event.event === 'Started') total.value = event.data.contentLength || 0
      if (event.event === 'Progress') downloaded.value += event.data.chunkLength
      if (event.event === 'Finished') message.value = '签名校验通过，正在启动安装程序…'
    }, { timeout: 10 * 60_000 })
  } catch (error) {
    message.value = `更新已中止：${String(error)}`
  } finally {
    installing.value = false
  }
}

onMounted(async () => {
  if (isTauri()) currentVersion.value = await getVersion().catch(() => currentVersion.value)
  await checkForUpdate(true)
})
</script>

<template>
  <section class="update-center" aria-labelledby="update-title">
    <header>
      <div><span>安全更新</span><h2 id="update-title">稳定频道</h2><p>更新包必须通过 Tauri 签名校验；SHA-256、大小和发布时间来自统一发布清单。</p></div>
      <button :disabled="checking || installing" @click="checkForUpdate(false)">{{ checking ? '检查中…' : '检查更新' }}</button>
    </header>
    <div class="update-version-row">
      <div><small>当前版本</small><b>v{{ currentVersion }}</b></div>
      <div><small>最新版本</small><b>v{{ manifest?.version || update?.version || currentVersion }}</b></div>
      <div><small>安装包大小</small><b>{{ fileSize }}</b></div>
      <div><small>发布时间</small><b>{{ manifest?.published_at ? new Date(manifest.published_at).toLocaleString('zh-CN') : '—' }}</b></div>
    </div>
    <dl v-if="manifest">
      <div><dt>频道</dt><dd>{{ manifest.channel || 'stable' }}</dd></div>
      <div><dt>平台</dt><dd>{{ manifest.platform || 'windows-x86_64' }}</dd></div>
      <div><dt>SHA-256</dt><dd>{{ manifest.sha256 || '等待发布' }}</dd></div>
    </dl>
    <p class="update-message">{{ message }}</p>
    <div v-if="installing" class="update-progress"><i :style="{ width: `${progress}%` }" /><span>{{ progress }}%</span></div>
    <button v-if="update" class="update-install" :disabled="installing" @click="installUpdate">{{ installing ? '正在下载…' : `下载并安装 v${update.version}` }}</button>
  </section>
</template>

<style scoped>
.update-center{display:grid;gap:16px;margin:18px 0;padding:22px;border:1px solid var(--tm-line);border-radius:16px;background:var(--tm-bg);color:var(--tm-ink)}
.update-center>header{display:flex;align-items:flex-start;justify-content:space-between;gap:20px}.update-center>header div{display:grid;gap:5px}.update-center h2{margin:0;font-size:18px}.update-center header span,.update-center small,.update-center dt{color:var(--tm-muted);font-size:10px}.update-center header p{max-width:68ch;margin:0;color:var(--tm-muted);font-size:11px;line-height:1.55}.update-center button{padding:10px 14px;border:1px solid var(--tm-line);border-radius:12px;background:var(--tm-surface);color:var(--tm-ink);font:inherit;font-size:11px;font-weight:650}.update-center button:disabled{cursor:not-allowed;opacity:.5}
.update-version-row{display:grid;grid-template-columns:repeat(4,minmax(0,1fr));gap:1px;overflow:hidden;border-radius:14px;background:var(--tm-line)}.update-version-row>div{display:grid;gap:5px;padding:15px;background:var(--tm-surface)}.update-version-row b{font-size:13px;font-variant-numeric:tabular-nums}
.update-center dl{display:grid;gap:1px;margin:0;overflow:hidden;border-radius:12px;background:var(--tm-line)}.update-center dl>div{display:grid;grid-template-columns:90px minmax(0,1fr);gap:12px;padding:10px 12px;background:var(--tm-surface)}.update-center dd{overflow:hidden;margin:0;font:10px/1.5 ui-monospace,SFMono-Regular,Consolas,monospace;text-overflow:ellipsis;white-space:nowrap}
.update-message{margin:0;color:var(--tm-muted);font-size:11px}.update-install{justify-self:start!important;border-color:var(--tm-ink)!important;background:var(--tm-ink)!important;color:var(--tm-on-ink)!important}.update-progress{position:relative;height:22px;overflow:hidden;border-radius:999px;background:var(--tm-surface)}.update-progress i{display:block;height:100%;border-radius:inherit;background:var(--tm-ink);transition:width .2s ease}.update-progress span{position:absolute;inset:0;display:grid;place-items:center;color:var(--tm-muted);font-size:9px;font-variant-numeric:tabular-nums;mix-blend-mode:difference}
@media(max-width:900px){.update-version-row{grid-template-columns:repeat(2,minmax(0,1fr))}}@media(max-width:620px){.update-center>header{flex-direction:column}.update-version-row{grid-template-columns:1fr}}
</style>
