import { ref, watch } from 'vue'

export type DescriptionVisibility = 'hover' | 'always' | 'hidden'
const storageKey = 'token-manager-description-visibility'

function readPreference(): DescriptionVisibility {
  const value = localStorage.getItem(storageKey)
  return value === 'always' || value === 'hidden' ? value : 'hover'
}

const visibility = ref<DescriptionVisibility>(readPreference())
const activeHelpId = ref<string | null>(null)

watch(visibility, value => {
  localStorage.setItem(storageKey, value)
  document.documentElement.dataset.descriptionVisibility = value
  activeHelpId.value = null
  window.dispatchEvent(new CustomEvent('token-manager-description-visibility', { detail: value }))
}, { immediate: true })

window.addEventListener('storage', event => {
  if (event.key !== storageKey || !event.newValue) return
  const value = event.newValue as DescriptionVisibility
  if (value === 'hover' || value === 'always' || value === 'hidden') visibility.value = value
})

export function closeActiveSupplementalHelp(id: string) {
  if (activeHelpId.value === id) activeHelpId.value = null
}

export function useDescriptionPreferences() {
  return {
    visibility,
    activeHelpId,
    setVisibility: (value: DescriptionVisibility) => { visibility.value = value },
    setActiveHelp: (value: string | null) => { activeHelpId.value = value },
  }
}
