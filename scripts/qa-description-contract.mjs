import { readFile, readdir } from 'node:fs/promises'
import { join } from 'node:path'

const root = process.cwd()
const preferences = await readFile(join(root, 'src/features/descriptionPreferences.ts'), 'utf8')
const help = await readFile(join(root, 'src/components/SupplementalHelp.vue'), 'utf8')
const css = await readFile(join(root, 'src/apple-design-system.css'), 'utf8')

const errors = []
if (/MutationObserver|classifyLegacyDescriptions|installSupplementalDescriptionObserver/.test(preferences)) errors.push('说明偏好仍包含运行时 DOM 猜测逻辑')
if (!/visibility\.value === 'hover'/.test(help) || !/v-if="interactive"/.test(help)) errors.push('SupplementalHelp 未严格限制为靠近模式交互')
if (!/data-description-visibility="hidden"[^\n]+display:none!important/.test(css)) errors.push('完全隐藏模式缺少不可交互的 display:none 规则')
if (/data-description-visibility="hidden"[^\n]+:where\(:hover/.test(css)) errors.push('完全隐藏模式仍可通过悬停恢复')
if (/tm-local-help-anchor/.test(css)) errors.push('已删除的死规则 tm-local-help-anchor 不应回归')
if (!css.includes(':where(:not(:hover):not(:focus-within))')) errors.push('靠近模式缺少“悬停才显现”的自我否定结构')
const hiddenRuleStart = css.indexOf('data-description-visibility="hidden"')
const hiddenRuleEnd = css.indexOf('data-description-visibility="hover"')
const hiddenBlock = hiddenRuleStart >= 0 && hiddenRuleEnd > hiddenRuleStart ? css.slice(hiddenRuleStart, hiddenRuleEnd) : ''
if (!/\.connection-state/.test(hiddenBlock)) errors.push('完全隐藏未覆盖数据通道状态说明')
if (!/\.floating-footer/.test(hiddenBlock)) errors.push('完全隐藏未覆盖悬浮窗状态说明')
if (!/\.settings-center/.test(hiddenBlock) || !/\.modal-backdrop/.test(hiddenBlock)) errors.push('完全隐藏缺少设置面板与模态框豁免')

async function walk(directory) {
  const paths = []
  for (const entry of await readdir(directory, { withFileTypes: true })) {
    const path = join(directory, entry.name)
    if (entry.isDirectory()) paths.push(...await walk(path))
    else if (entry.name.endsWith('.vue')) paths.push(path)
  }
  return paths
}

for (const path of await walk(join(root, 'src'))) {
  const source = await readFile(path, 'utf8')
  if (/classifyLegacyDescriptions|installSupplementalDescriptionObserver/.test(source)) errors.push(`${path}: 禁止重新接入旧说明观察器`)
}

if (errors.length) {
  console.error(errors.map(item => `- ${item}`).join('\n'))
  process.exit(1)
}
console.log('说明系统静态契约通过：无 DOM 猜测、隐藏模式不可唤出、局部帮助为单实例。')
