import { ref, watch } from 'vue'

const motionEnabled = ref(localStorage.getItem('token-manager-motion-enabled') !== '0')
const particlesEnabled = ref(localStorage.getItem('token-manager-particles-enabled') !== '0')
const springMotionEnabled = ref(localStorage.getItem('token-manager-spring-motion-enabled') !== '0')
const spotlightEnabled = ref(localStorage.getItem('token-manager-spotlight-enabled') !== '0')

watch(motionEnabled, value => {
  localStorage.setItem('token-manager-motion-enabled', value ? '1' : '0')
  document.documentElement.classList.toggle('motion-off', !value)
}, { immediate: true })

watch(particlesEnabled, value => {
  localStorage.setItem('token-manager-particles-enabled', value ? '1' : '0')
})

watch(springMotionEnabled, value => {
  localStorage.setItem('token-manager-spring-motion-enabled', value ? '1' : '0')
  document.documentElement.classList.toggle('spring-motion-on', value)
  document.documentElement.classList.toggle('spring-motion-off', !value)
}, { immediate: true })

watch(spotlightEnabled, value => {
  localStorage.setItem('token-manager-spotlight-enabled', value ? '1' : '0')
  document.documentElement.classList.toggle('spotlight-on', value)
  document.documentElement.classList.toggle('spotlight-off', !value)
}, { immediate: true })

export function useMotionPreferences() {
  function applySpotlight(value: boolean, notify = true) {
    spotlightEnabled.value = value
    if (notify) window.dispatchEvent(new CustomEvent('token-manager-spotlight-change', { detail: value }))
  }
  return {
    motionEnabled,
    particlesEnabled,
    springMotionEnabled,
    spotlightEnabled,
    setMotionEnabled: (value: boolean) => { motionEnabled.value = value },
    setParticlesEnabled: (value: boolean) => { particlesEnabled.value = value },
    setSpringMotionEnabled: (value: boolean) => { springMotionEnabled.value = value },
    setSpotlightEnabled: (value: boolean) => { applySpotlight(value, true) },
    syncSpotlightEnabled: (value: boolean) => { applySpotlight(value, false) },
  }
}
