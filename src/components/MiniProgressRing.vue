<script setup lang="ts">
import { computed } from 'vue'

const props = withDefaults(defineProps<{
  value: number
  label: string
  tone?: 'default' | 'critical'
  ariaLabel?: string
}>(), {
  tone: 'default',
  ariaLabel: ''
})

const normalizedValue = computed(() => Math.max(0, Math.min(100, Number(props.value) || 0)))
const circumference = 2 * Math.PI * 20
const dashOffset = computed(() => circumference * (1 - normalizedValue.value / 100))
</script>

<template>
  <span
    class="mini-progress-ring"
    :class="`tone-${tone}`"
    role="img"
    :aria-label="ariaLabel || `用量指标 ${label}`"
  >
    <svg viewBox="0 0 48 48" aria-hidden="true">
      <circle class="ring-track" cx="24" cy="24" r="20" />
      <circle
        class="ring-value"
        cx="24"
        cy="24"
        r="20"
        :style="{ strokeDasharray: circumference, strokeDashoffset: dashOffset }"
      />
    </svg>
    <b :title="label">{{ label }}</b>
  </span>
</template>

<style scoped>
.mini-progress-ring {
  position: relative;
  display: grid;
  width: 3rem;
  height: 3rem;
  place-items: center;
  flex: 0 0 auto;
  color: var(--tm-chart-ring, var(--tm-ink, #1d1d1f));
  transition: transform 180ms cubic-bezier(.22, 1, .36, 1);
}
.mini-progress-ring svg {
  position: absolute;
  inset: 0;
  display: block;
  width: 100%;
  height: 100%;
  overflow: visible;
  transform: rotate(-90deg);
}
.mini-progress-ring circle {
  fill: none;
  stroke-width: 3.75;
  vector-effect: non-scaling-stroke;
}
.ring-track { stroke: var(--tm-line, #e5e5ea); }
.ring-value {
  stroke: currentColor;
  stroke-linecap: round;
  transition: stroke-dashoffset 300ms cubic-bezier(.22, 1, .36, 1);
}
.mini-progress-ring::after {
  content: '';
  position: absolute;
  inset: .43rem;
  border: 1px solid color-mix(in srgb, var(--tm-ink, #1d1d1f) 5%, transparent);
  border-radius: 50%;
  background: color-mix(in srgb, var(--tm-bg, #fff) 92%, transparent);
}
.mini-progress-ring b {
  position: relative;
  z-index: 1;
  max-width: 2rem;
  overflow: hidden;
  color: var(--tm-ink, #1d1d1f);
  font-family: var(--apple-font);
  font-size: .5625rem;
  font-weight: 700;
  font-variant-numeric: tabular-nums;
  line-height: 1;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.tone-critical { color: #ff3b30; }
@media (prefers-reduced-motion: reduce) {
  .mini-progress-ring,
  .ring-value { transition: none; }
}
</style>
