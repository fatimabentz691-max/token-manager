<template>
  <span class="proxy-gateway-icon" aria-hidden="true">
    <svg viewBox="0 0 24 24" fill="none" focusable="false">
      <rect x="3.25" y="3.25" width="17.5" height="17.5" rx="5.25" />
      <path d="M7.4 8.1h2.2c1.35 0 2.1.66 2.78 1.82l2.06 3.56c.5.86 1.06 1.42 2.16 1.42h.1" />
      <path d="M7.4 15.9h2.2c1.35 0 2.1-.66 2.78-1.82l2.06-3.56c.5-.86 1.06-1.42 2.16-1.42h.1" />
      <circle cx="7.35" cy="8.1" r="1.05" />
      <circle cx="7.35" cy="15.9" r="1.05" />
      <circle cx="16.65" cy="8.1" r="1.05" />
      <circle cx="16.65" cy="15.9" r="1.05" />
    </svg>
  </span>
</template>

<style scoped>
.proxy-gateway-icon {
  display: inline-grid;
  width: 1.625rem;
  height: 1.625rem;
  flex: 0 0 1.625rem;
  place-items: center;
  border-radius: .5625rem;
  color: currentColor;
}

svg {
  width: 100%;
  height: 100%;
  overflow: visible;
}

rect,
path {
  stroke: currentColor;
  stroke-width: 1.45;
  stroke-linecap: round;
  stroke-linejoin: round;
}

circle {
  fill: currentColor;
}
</style>
