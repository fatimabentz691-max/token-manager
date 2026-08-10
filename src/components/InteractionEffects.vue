<script setup lang="ts">
import { animate, type AnimationPlaybackControlsWithThen } from 'motion'
import { onMounted, onUnmounted, ref, watch } from 'vue'
import { useMotionPreferences } from '../features/motionPreferences'

interface ClickParticle {
  id: number
  x: number
  y: number
  dx: number
  dy: number
  size: number
  color: string
  delay: number
}

const particles = ref<ClickParticle[]>([])
const { motionEnabled, particlesEnabled, springMotionEnabled, spotlightEnabled } = useMotionPreferences()
let particleId = 0
let activeSurface: HTMLElement | null = null
let pressedControl: HTMLElement | null = null
let pressedPointerId = -1
let activeSurfaceBounds: DOMRect | null = null
const surfaceAnimations = new WeakMap<HTMLElement, AnimationPlaybackControlsWithThen>()
const controlAnimations = new WeakMap<HTMLElement, AnimationPlaybackControlsWithThen>()
const animatedSurfaces = new Set<HTMLElement>()
const animatedControls = new Set<HTMLElement>()
let spotlightFrame = 0
let pointerX = 50
let pointerY = 50
let renderedX = 50
let renderedY = 50
let velocityX = 0
let velocityY = 0

// 始终选择事件路径中最内层的可视卡片，避免父卡片和子卡片同时出现微光。
const surfaceSelector = [
  '.card', '.provider-overview', '.dashboard-empty', '.visual-card', '.model-bill',
  '.settings-nav', '.settings-detail', '.command-center', '.retention-panel',
  '.conversion-strip', '.all-overview-grid > section', '.all-overview-grid > aside',
  '.monitor-status-bar', '.dashboard-switcher', '.floating-dashboard-item',
  '.status-capsule', '.capsule-detail', '.remote-content', '.prompt-card',
  '.arena-card', '.report-heading', '.floating-control-card', '.floating-preview-card',
  '.saved-accounts > div',
].join(',')

function stopSpotlightFrame() {
  if (spotlightFrame) cancelAnimationFrame(spotlightFrame)
  spotlightFrame = 0
}

function resetSurfaceMotion(surface: HTMLElement) {
  surfaceAnimations.get(surface)?.stop()
  surfaceAnimations.delete(surface)
  animatedSurfaces.delete(surface)
  surface.classList.remove('tm-motion-surface')
  surface.style.removeProperty('--tm-surface-lift')
  surface.style.removeProperty('--tm-surface-scale')
}

function animateSurface(surface: HTMLElement, entering: boolean) {
  if (!motionEnabled.value || !springMotionEnabled.value) {
    resetSurfaceMotion(surface)
    return
  }
  surfaceAnimations.get(surface)?.stop()
  surface.classList.add('tm-motion-surface')
  animatedSurfaces.add(surface)

  // Motion 的物理弹簧会从元素当前状态继续计算；每张卡片持有自己的控制器，
  // 鼠标从 A 移到 B 时不再取消 A 的复位动画。
  const controls = animate(
    surface,
    {
      '--tm-surface-lift': entering ? '-1.15px' : '0px',
      '--tm-surface-scale': entering ? '1.0015' : '1',
    },
    {
      type: 'spring',
      stiffness: entering ? 520 : 610,
      damping: entering ? 42 : 46,
      mass: .58,
      restSpeed: .08,
      restDelta: .015,
    },
  )
  surfaceAnimations.set(surface, controls)
  const finishSurface = () => {
    if (surfaceAnimations.get(surface) !== controls) return
    surfaceAnimations.delete(surface)
    animatedSurfaces.delete(surface)
    if (!entering) resetSurfaceMotion(surface)
  }
  void controls.finished.then(finishSurface, finishSurface)
}

function renderSpotlight() {
  if (!activeSurface || !spotlightEnabled.value) {
    stopSpotlightFrame()
    return
  }
  // 轻阻尼弹簧插值让微光跟随鼠标时保留少量惯性，而不是瞬间跳变。
  velocityX = (velocityX + (pointerX - renderedX) * .16) * .7
  velocityY = (velocityY + (pointerY - renderedY) * .16) * .7
  renderedX += velocityX
  renderedY += velocityY
  activeSurface.style.setProperty('--tm-card-pointer-x', `${renderedX}%`)
  activeSurface.style.setProperty('--tm-card-pointer-y', `${renderedY}%`)
  if (Math.abs(pointerX - renderedX) > .02 || Math.abs(pointerY - renderedY) > .02 || Math.abs(velocityX) > .02 || Math.abs(velocityY) > .02) {
    spotlightFrame = requestAnimationFrame(renderSpotlight)
  } else {
    spotlightFrame = 0
  }
}

function clearActiveSurface() {
  if (!activeSurface) return
  const previous = activeSurface
  previous.classList.remove('tm-spotlight-active', 'tm-spring-surface')
  previous.style.removeProperty('--tm-card-pointer-x')
  previous.style.removeProperty('--tm-card-pointer-y')
  animateSurface(previous, false)
  activeSurface = null
  activeSurfaceBounds = null
  stopSpotlightFrame()
}

function onPointerMove(event: PointerEvent) {
  if (!motionEnabled.value) {
    clearActiveSurface()
    return
  }
  const target = event.target instanceof Element ? event.target : null
  const surface = target?.closest<HTMLElement>(surfaceSelector) || null
  if (surface !== activeSurface) {
    clearActiveSurface()
    activeSurface = surface
    if (activeSurface) {
      activeSurface.classList.add('tm-spring-surface')
      activeSurface.classList.toggle('tm-spotlight-active', spotlightEnabled.value)
      renderedX = pointerX = 50
      renderedY = pointerY = 50
      velocityX = velocityY = 0
      activeSurfaceBounds = activeSurface.getBoundingClientRect()
      animateSurface(activeSurface, true)
    }
  }
  if (!activeSurface) return
  if (!spotlightEnabled.value) {
    activeSurface.classList.remove('tm-spotlight-active')
    activeSurface.style.removeProperty('--tm-card-pointer-x')
    activeSurface.style.removeProperty('--tm-card-pointer-y')
    stopSpotlightFrame()
    return
  }
  const bounds = activeSurfaceBounds || activeSurface.getBoundingClientRect()
  activeSurfaceBounds = bounds
  pointerX = Math.max(0, Math.min(100, ((event.clientX - bounds.left) / Math.max(1, bounds.width)) * 100))
  pointerY = Math.max(0, Math.min(100, ((event.clientY - bounds.top) / Math.max(1, bounds.height)) * 100))
  if (!spotlightFrame) spotlightFrame = requestAnimationFrame(renderSpotlight)
}

function onPointerOut(event: PointerEvent) {
  if (!event.relatedTarget) clearActiveSurface()
}

function invalidateSurfaceBounds() {
  activeSurfaceBounds = null
}

function resetControlMotion(control: HTMLElement) {
  controlAnimations.get(control)?.stop()
  controlAnimations.delete(control)
  animatedControls.delete(control)
  control.classList.remove('tm-motion-control')
  control.style.removeProperty('--tm-control-scale')
}

function animateControl(control: HTMLElement, pressed: boolean) {
  if (!motionEnabled.value || !springMotionEnabled.value) {
    resetControlMotion(control)
    return
  }
  controlAnimations.get(control)?.stop()
  control.classList.add('tm-motion-control')
  animatedControls.add(control)
  const controls = animate(
    control,
    { '--tm-control-scale': pressed ? '.982' : '1' },
    {
      type: 'spring',
      stiffness: pressed ? 980 : 720,
      damping: pressed ? 58 : 44,
      mass: .42,
      restSpeed: .1,
      restDelta: .002,
    },
  )
  controlAnimations.set(control, controls)
  const finishControl = () => {
    if (controlAnimations.get(control) !== controls) return
    controlAnimations.delete(control)
    animatedControls.delete(control)
    if (!pressed) resetControlMotion(control)
  }
  void controls.finished.then(finishControl, finishControl)
}

function animatePress(target: Element | null, pointerId: number) {
  if (!motionEnabled.value || !springMotionEnabled.value) return
  const control = target?.closest<HTMLElement>('button,[role="button"],input[type="checkbox"],input[type="radio"],select')
  if (!control) return
  if (pressedControl && pressedControl !== control) animateControl(pressedControl, false)
  pressedControl = control
  pressedPointerId = pointerId
  animateControl(control, true)
}

function releasePress(event: PointerEvent) {
  if (!pressedControl || (pressedPointerId >= 0 && event.pointerId !== pressedPointerId)) return
  const control = pressedControl
  pressedControl = null
  pressedPointerId = -1
  animateControl(control, false)
}

// 点击粒子只在用户主动交互时短暂出现，不建立常驻渲染循环。
function onPointerDown(event: PointerEvent) {
  const target = event.target instanceof Element ? event.target : null
  if (event.button !== 0) return
  animatePress(target, event.pointerId)
  if (!motionEnabled.value || !particlesEnabled.value) return
  if (!target?.closest('.shell,.floating-shell,.prompt-modal-backdrop,.arena-modal-backdrop')) return

  const themeRoot = target.closest<HTMLElement>('.shell,.floating-shell')
  const color = themeRoot
    ? getComputedStyle(themeRoot).getPropertyValue('--tm-accent').trim() || '#1d1d1f'
    : '#1d1d1f'
  const created: ClickParticle[] = Array.from({ length: 7 }, (_, index) => {
    const angle = (Math.PI * 2 * index) / 7 - Math.PI / 2
    const distance = 17 + (index % 3) * 5
    return {
      id: ++particleId,
      x: event.clientX,
      y: event.clientY,
      dx: Math.cos(angle) * distance,
      dy: Math.sin(angle) * distance,
      size: 3 + (index % 2),
      color,
      delay: index * 8,
    }
  })
  particles.value = [...particles.value.slice(-35), ...created]
  const ids = new Set(created.map(item => item.id))
  window.setTimeout(() => {
    particles.value = particles.value.filter(item => !ids.has(item.id))
  }, 560)
}

watch([motionEnabled, springMotionEnabled], ([motion, spring]) => {
  if (!motion || !spring) {
    for (const surface of [...animatedSurfaces]) resetSurfaceMotion(surface)
    for (const control of [...animatedControls]) resetControlMotion(control)
    pressedControl = null
    pressedPointerId = -1
    if (!motion) clearActiveSurface()
  }
})
watch(spotlightEnabled, enabled => {
  if (!activeSurface) return
  activeSurface.classList.toggle('tm-spotlight-active', enabled && motionEnabled.value)
  if (!enabled) {
    activeSurface.style.removeProperty('--tm-card-pointer-x')
    activeSurface.style.removeProperty('--tm-card-pointer-y')
    stopSpotlightFrame()
  }
})

onMounted(() => {
  document.addEventListener('pointerdown', onPointerDown, { passive: true })
  document.addEventListener('pointerup', releasePress, { passive: true })
  document.addEventListener('pointercancel', releasePress, { passive: true })
  document.addEventListener('pointermove', onPointerMove, { passive: true })
  document.addEventListener('pointerout', onPointerOut, { passive: true })
  window.addEventListener('resize', invalidateSurfaceBounds, { passive: true })
  window.addEventListener('scroll', invalidateSurfaceBounds, { passive: true, capture: true })
})
onUnmounted(() => {
  document.removeEventListener('pointerdown', onPointerDown)
  document.removeEventListener('pointerup', releasePress)
  document.removeEventListener('pointercancel', releasePress)
  document.removeEventListener('pointermove', onPointerMove)
  document.removeEventListener('pointerout', onPointerOut)
  window.removeEventListener('resize', invalidateSurfaceBounds)
  window.removeEventListener('scroll', invalidateSurfaceBounds, true)
  clearActiveSurface()
  for (const surface of [...animatedSurfaces]) resetSurfaceMotion(surface)
  for (const control of [...animatedControls]) resetControlMotion(control)
})
</script>

<template>
  <Teleport to="body">
    <div class="interaction-effects" aria-hidden="true">
      <i
        v-for="particle in particles"
        :key="particle.id"
        :style="{
          left: `${particle.x}px`,
          top: `${particle.y}px`,
          width: `${particle.size}px`,
          height: `${particle.size}px`,
          background: particle.color,
          boxShadow: `0 0 9px ${particle.color}`,
          animationDelay: `${particle.delay}ms`,
          '--particle-x': `${particle.dx}px`,
          '--particle-y': `${particle.dy}px`,
        }"
      ></i>
    </div>
  </Teleport>
</template>

<style scoped>
.interaction-effects{position:fixed;z-index:10000;inset:0;overflow:hidden;pointer-events:none}
.interaction-effects i{position:absolute;border-radius:50%;transform:translate(-50%,-50%);animation:interaction-particle .46s cubic-bezier(.2,.75,.25,1) both}
@keyframes interaction-particle{
  0%{opacity:0;transform:translate(-50%,-50%) scale(.45)}
  18%{opacity:.82}
  100%{opacity:0;transform:translate(calc(-50% + var(--particle-x)),calc(-50% + var(--particle-y))) scale(.1)}
}
@media(prefers-reduced-motion:reduce){.interaction-effects{display:none}}
:global(.motion-off) .interaction-effects{display:none}
</style>
