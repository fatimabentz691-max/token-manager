import { currentCloudBaseUrl } from './cloudConfig'

export interface ReleaseManifestV2 {
  version: string
  channel: string
  platform: string
  title?: string
  size_bytes: number
  sha256: string
  download_url: string
  updater_url?: string
  notes: string
  highlights?: string[]
  fixes?: string[]
  published_at: string
}

function versionParts(value: string) {
  return value.replace(/^v/i, '').split(/[+-]/, 1)[0].split('.').map(part => Number.parseInt(part, 10) || 0)
}

export function compareVersions(left: string, right: string) {
  const a = versionParts(left)
  const b = versionParts(right)
  for (let index = 0; index < Math.max(a.length, b.length); index += 1) {
    const difference = (a[index] || 0) - (b[index] || 0)
    if (difference) return difference
  }
  return 0
}

function normalizeLines(value: unknown, limit = 4) {
  if (Array.isArray(value)) return value.map(item => String(item).trim()).filter(Boolean).slice(0, limit)
  return String(value || '').split(/\r?\n|[；;]/).map(item => item.replace(/^[-*•\d.、\s]+/, '').trim()).filter(Boolean).slice(0, limit)
}

export function releaseSummary(manifest: ReleaseManifestV2 | null) {
  if (!manifest) return { highlights: [] as string[], fixes: [] as string[] }
  const highlights = normalizeLines(manifest.highlights)
  const fixes = normalizeLines(manifest.fixes)
  if (highlights.length || fixes.length) return { highlights, fixes }
  const notes = normalizeLines(manifest.notes, 4)
  return {
    highlights: notes.filter(item => !/(修复|解决|改正|问题)/.test(item)).slice(0, 3),
    fixes: notes.filter(item => /(修复|解决|改正|问题)/.test(item)).slice(0, 3),
  }
}

export async function fetchReleaseManifest(signal?: AbortSignal) {
  const response = await fetch(`${currentCloudBaseUrl()}/v1/release/latest`, { cache: 'no-store', signal })
  if (!response.ok) throw new Error(`发布清单请求失败：HTTP ${response.status}`)
  return await response.json() as ReleaseManifestV2
}

export function trackedDownloadUrl(currentVersion: string) {
  const params = new URLSearchParams({ source: 'app-update', from_version: currentVersion })
  return `${currentCloudBaseUrl()}/v1/download/latest?${params}`
}
