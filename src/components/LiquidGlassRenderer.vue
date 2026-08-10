<script setup lang="ts">
import { nextTick, onMounted, onUnmounted, ref, watch } from 'vue'
import { useMotionPreferences } from '../features/motionPreferences'
import { useThemePreferences } from '../features/themePreferences'
import type { GlassDistortionSettings, GlassQuality } from '../features/visualPreferences'

const props = withDefaults(defineProps<{ quality?: GlassQuality; distortion: GlassDistortionSettings; compact?: boolean; backgroundUrl?: string; videoUrl?: string }>(), {
  quality: 'high',
  compact: false,
  backgroundUrl: '',
  videoUrl: '',
})
const emit = defineEmits<{ videoError: [message: string] }>()

const canvas = ref<HTMLCanvasElement | null>(null)
const videoSource = ref<HTMLVideoElement | null>(null)
const { motionEnabled } = useMotionPreferences()
const { liquidTone, liquidTransparency } = useThemePreferences()

/**
 * Windows 主窗口启用双通道 WebGL 真实折射。
 * 透明悬浮窗会被 WebView2 提升为独立 DirectComposition 表面，因此继续使用
 * CSS/SVG 安全折射，避免透明窗口与全屏 GPU 画布组合后出现黑屏。
 */
const isWindowsRuntime = typeof navigator !== 'undefined' && /Windows/i.test(navigator.userAgent)
const allowFullScreenWebGl = !(isWindowsRuntime && props.compact)

const MAX_SURFACES = 80
const SURFACE_SELECTOR = [
  '[data-liquid-surface]',
  '.card', '.glass-card', '.command-status', '.provider-overview', '.dashboard-empty', '.visual-card', '.model-bill',
  '.settings-nav', '.settings-detail', '.command-center', '.retention-panel',
  '.conversion-strip', '.all-overview-grid > section', '.all-overview-grid > aside',
  '.monitor-status-bar', '.dashboard-switcher', '.floating-dashboard-item',
  '.status-capsule', '.capsule-detail', '.remote-content', '.prompt-card',
  '.arena-card', '.report-heading', '.floating-control-card', '.floating-preview-card',
  '.saved-accounts > div', '.theme-liquid-glass.shell > aside',
  '.theme-liquid-glass .app-titlebar', '.theme-liquid-glass .content > header', '.theme-liquid-glass.floating-shell',
  '.model-kpis > article', '.deepseek-metrics > article', '.codex-kpis > article',
  '.token-chart-summary > span', '.capsule-stats > span', '.capsule-chart',
  '.floating-dashboard-chart', '.floating-dashboard-extras > span', '.floating-dashboard-footer',
  '.connection-state', '.cc-switch-state', '.prompt-stats > *', '.model-stat-grid > article',
  '.all-summary', '.balance-primary', '.token-primary', '.quality-panel',
  '.arena-source', '.arena-commandbar', '.arena-table', '.arena-detail',
  '.prompt-categories', '.prompt-toolbar > label', '.today-model-item',
  '.floating-focus-summary', '.floating-focus-chart', '.floating-focus-stats > span',
  '.floating-chart-tabs', '.floating-chart-empty', '.sidebar-material-footer',
].join(',')

/*
 * 玻璃材质必须只有一层清晰边界。父卡片已经是折射面时，内部的按钮、状态块和文字容器
 * 只负责承载内容，不能再次参与 GPU 折射，否则会出现用户截图中的透明矩形残影。
 */
const GROUP_ONLY_SELECTOR = [
  '.command-center', '.retention-panel', '.dashboard-context-heading', '.arena-page-heading',
].join(',')

const VERTEX_SHADER = `#version 300 es
in vec2 aPosition;
out vec2 vUv;
void main() {
  vUv = aPosition * .5 + .5;
  gl_Position = vec4(aPosition, 0.0, 1.0);
}`

const ENVIRONMENT_SHADER = `#version 300 es
precision highp float;
in vec2 vUv;
out vec4 outColor;
uniform vec2 uResolution;
uniform vec2 uPointer;
uniform float uTime;
uniform float uMotion;
uniform vec3 uAccent;
uniform vec3 uGlow;
uniform float uClearTone;
uniform sampler2D uBackground;
uniform float uHasBackground;
uniform vec2 uBackgroundSize;

float hash21(vec2 p) {
  p = fract(p * vec2(123.34, 456.21));
  p += dot(p, p + 45.32);
  return fract(p.x * p.y);
}

float noise(vec2 p) {
  vec2 i = floor(p);
  vec2 f = fract(p);
  f = f * f * (3.0 - 2.0 * f);
  return mix(mix(hash21(i), hash21(i + vec2(1, 0)), f.x),
             mix(hash21(i + vec2(0, 1)), hash21(i + vec2(1)), f.x), f.y);
}

void main() {
  vec2 uv = vUv;
  vec2 aspect = vec2(uResolution.x / max(uResolution.y, 1.0), 1.0);
  vec2 pointer = uPointer;
  float time = uTime * uMotion;

  vec3 darkBase = mix(vec3(.012, .018, .040), vec3(.026, .040, .086), uv.y);
  vec3 clearBase = mix(vec3(.91, .94, .98), vec3(.992, .996, 1.0), uv.y);
  vec3 base = mix(darkBase, clearBase, uClearTone);
  if (uHasBackground > .5) {
    float viewportAspect = uResolution.x / max(uResolution.y, 1.0);
    float imageAspect = uBackgroundSize.x / max(uBackgroundSize.y, 1.0);
    vec2 coverScale = imageAspect > viewportAspect
      ? vec2(viewportAspect / imageAspect, 1.0)
      : vec2(1.0, imageAspect / viewportAspect);
    vec2 backgroundUv = clamp((uv - .5) * coverScale + .5, 0.0, 1.0);
    vec3 photograph = texture(uBackground, backgroundUv).rgb;
    float luminance = dot(photograph, vec3(.2126, .7152, .0722));
    photograph = mix(photograph, photograph / max(luminance, .24) * .42, .10);
    vec3 darkPhotograph = photograph * mix(vec3(.68), vec3(.84), uv.y);
    vec3 clearPhotograph = mix(photograph, vec3(luminance), .045);
    clearPhotograph = mix(clearPhotograph, vec3(1.0), .06);
    base = mix(mix(darkPhotograph, darkBase, .18), mix(clearPhotograph, clearBase, .08), uClearTone);
  }
  float blueHalo = exp(-length((uv - vec2(.18 + sin(time * .11) * .05, .78)) * aspect) * 3.25);
  float purpleHalo = exp(-length((uv - vec2(.82, .18 + cos(time * .09) * .05)) * aspect) * 3.5);
  float cursorHalo = exp(-length((uv - pointer) * aspect) * 5.4);

  vec2 flowUv = uv * vec2(3.2, 2.4);
  float flow = noise(flowUv + vec2(time * .028, -time * .019));
  flow += noise(flowUv * 2.1 - vec2(time * .017, time * .024)) * .5;
  flow = smoothstep(.58, 1.32, flow);

  vec3 color = base;
  float ambientStrength = mix(1.0, .38, uClearTone);
  color += uAccent * blueHalo * .30 * ambientStrength;
  color += uGlow * purpleHalo * .23 * ambientStrength;
  color += mix(uAccent, vec3(.64, .92, 1.0), .48) * cursorHalo * .12;
  color += mix(uAccent, uGlow, uv.x) * flow * .035;

  /*
   * 低亮度环境光丝不是装饰线：它们为透镜提供连续的光学参照物。
   * 光丝进入玻璃表面后会被第二通道真实弯折，因此肉眼和截图差分都能辨认位移。
   */
  float filamentA = exp(
    -abs(uv.y - (.33 + sin(uv.x * 7.2 + time * .08) * .052)) * 84.0
  );
  float filamentB = exp(
    -abs(uv.x - (.70 + sin(uv.y * 8.4 - time * .065) * .043)) * 96.0
  );
  float filamentC = exp(
    -abs(uv.y - (.76 + sin(uv.x * 5.1 - time * .052) * .036)) * 118.0
  );
  color += mix(vec3(.70, .75, .82), uAccent, .14) * filamentA * .072;
  color += mix(vec3(.62, .68, .77), uGlow, .10) * filamentB * .058;
  color += vec3(.74, .78, .84) * filamentC * .038;

  /*
   * 连续的软焦光学纹理让每一块玻璃都拥有可折弯信息。
   * 两个不同尺度的平滑噪声相减形成天然亮度边界，不会出现网格或静态贴图感。
   */
  vec2 opticalWarp = vec2(
    noise(uv * vec2(4.2, 3.1) + vec2(time * .010, 0.0)),
    noise(uv * vec2(3.4, 4.7) - vec2(0.0, time * .009))
  ) - .5;
  float opticalBroad = noise(uv * vec2(46.0, 30.0) + opticalWarp * 2.6);
  float opticalFine = noise(uv * vec2(92.0, 60.0) - opticalWarp * 3.2);
  float opticalDetail = (opticalBroad - .5) * .72 + (opticalFine - .5) * .28;
  color += mix(vec3(.58, .64, .74), uAccent, .08) * opticalDetail * .045;

  /*
   * 指针附近只有一圈宽而柔和的内部照明，不铺满界面。
   * 它随输入移动，并在当前玻璃中被再次折射，形成 Apple 式接触点能量反馈。
   */
  float pointerDistance = length((uv - pointer) * aspect);
  float pointerRing = exp(-abs(pointerDistance - .064) * 58.0);
  color += mix(vec3(.82, .88, .96), uAccent, .13) * pointerRing * .15;

  /* 极淡的离散环境反光点同样参与折射，但不会形成常驻彩色粒子特效。 */
  vec2 cellUv = uv * vec2(54.0, 34.0);
  vec2 cell = floor(cellUv);
  vec2 cellPoint = vec2(hash21(cell), hash21(cell + 17.41));
  float sparkle = 1.0 - smoothstep(.018, .075, length(fract(cellUv) - cellPoint));
  sparkle *= step(.86, hash21(cell + 7.13));
  color += vec3(.74, .79, .88) * sparkle * .075;

  float vignette = smoothstep(1.18, .20, length((uv - .5) * vec2(1.05, .88)));
  color *= mix(.72, 1.05, vignette);
  outColor = vec4(color, 1.0);
}`

const COMPOSITE_SHADER = `#version 300 es
precision highp float;
in vec2 vUv;
out vec4 outColor;
uniform sampler2D uScene;
uniform vec2 uResolution;
uniform vec2 uPointer;
uniform float uTime;
uniform float uQuality;
uniform float uRefractionEnabled;
uniform float uDispersionEnabled;
uniform float uDistortionIntensity;
uniform float uEdgeStrength;
uniform float uMagnificationStrength;
uniform float uDispersionStrength;
uniform float uMaterialVisibility;
uniform int uRectCount;
uniform vec4 uRects[${MAX_SURFACES}];
/* x = 圆角半径，y = 光学材质类型（1 代表整条标题栏强弧形透镜）。 */
uniform vec2 uSurfaceMeta[${MAX_SURFACES}];

vec2 safeNormalize(vec2 value) {
  return value * inversesqrt(max(dot(value, value), 1e-8));
}

float roundedRectDistance(vec2 uv, vec4 rect, float radius) {
  vec2 center = rect.xy + rect.zw * .5;
  vec2 p = (uv - center) * uResolution;
  vec2 halfSize = rect.zw * uResolution * .5;
  float safeRadius = clamp(radius, 0.0, min(halfSize.x, halfSize.y));
  vec2 q = abs(p) - max(halfSize - vec2(safeRadius), vec2(0.0));
  return length(max(q, 0.0)) + min(max(q.x, q.y), 0.0) - safeRadius;
}

vec2 roundedRectNormal(vec2 uv, vec4 rect, float radius) {
  vec2 texel = 1.0 / uResolution;
  float left = roundedRectDistance(uv - vec2(texel.x, 0.0), rect, radius);
  float right = roundedRectDistance(uv + vec2(texel.x, 0.0), rect, radius);
  float bottom = roundedRectDistance(uv - vec2(0.0, texel.y), rect, radius);
  float top = roundedRectDistance(uv + vec2(0.0, texel.y), rect, radius);
  return safeNormalize(vec2(right - left, top - bottom));
}

void main() {
  vec2 uv = vUv;
  int surface = -1;
  float distanceToEdge = 9999.0;
  float smallestArea = 9999.0;

  for (int i = 0; i < ${MAX_SURFACES}; i++) {
    if (i >= uRectCount) break;
    vec4 candidate = uRects[i];
    if (
      uv.x < candidate.x ||
      uv.y < candidate.y ||
      uv.x > candidate.x + candidate.z ||
      uv.y > candidate.y + candidate.w
    ) continue;
    float d = roundedRectDistance(uv, candidate, uSurfaceMeta[i].x);
    float area = candidate.z * candidate.w;
    if (d <= 0.0 && area < smallestArea) {
      surface = i;
      distanceToEdge = d;
      smallestArea = area;
    }
  }

  if (surface < 0) {
    outColor = texture(uScene, uv);
    return;
  }

  vec4 rect = uRects[surface];
  float radius = uSurfaceMeta[surface].x;
  float titlebarSurface = step(.5, uSurfaceMeta[surface].y);
  vec2 local = clamp((uv - rect.xy) / rect.zw, 0.0, 1.0);
  vec2 center = rect.xy + rect.zw * .5;
  vec2 texel = 1.0 / uResolution;

  float depth = max(-distanceToEdge, 0.0);
  float minimumSide = min(rect.z * uResolution.x, rect.w * uResolution.y);
  float bevelWidth = clamp(minimumSide * .21, 14.0, 40.0);
  float substantial = smoothstep(92.0, 360.0, minimumSide);

  /*
   * rim 在真实内边缘为 1，向玻璃内部衰减。旧实现方向相反，
   * 使 Fresnel 与折射厚边落在了卡片中心；这里按圆角 SDF 深度修正。
   */
  float rim = 1.0 - smoothstep(.75, bevelWidth, depth);
  float interior = smoothstep(3.0, bevelWidth * 1.18, depth);
  float nonlinearRim = pow(clamp(rim, 0.0, 1.0), 1.42);
  vec2 edgeNormal = roundedRectNormal(uv, rect, radius);

  /*
   * 先对整块玻璃做轻微局部放大，再叠加以像素为单位的非线性边缘弯折。
   * 以像素计算可以避免宽屏下横纵位移强度不一致。
   */
  float thickness = mix(.82, 1.28, substantial);
  float magnification = mix(.014, .038, uQuality)
    * interior * thickness * uMagnificationStrength * uDistortionIntensity;
  /* 标题栏是一整条连续圆弧透镜，中央和两端都参与放大，不留下平直过渡区。 */
  float titlebarArc = .38 + .62 * (1.0 - pow(abs(local.x * 2.0 - 1.0), 2.0));
  float titlebarMagnification = (.052 + .022 * titlebarArc)
    * uMagnificationStrength * uDistortionIntensity;
  magnification = mix(magnification, titlebarMagnification, titlebarSurface);
  vec2 sampleUv = mix(uv, center, magnification * uRefractionEnabled);

  float edgeBendPx = mix(10.0, 68.0, uQuality)
    * nonlinearRim * thickness * uEdgeStrength * uDistortionIntensity;
  float bodyBendPx = mix(1.0, 3.8, uQuality)
    * interior * thickness * uDistortionIntensity;
  vec2 displacementPx = edgeNormal * (edgeBendPx + bodyBendPx);

  /*
   * 整条标题栏使用同一个椭圆弧形位移场：横向从左至右连续，纵向贯穿整条玻璃。
   * 四个现有设置参数仍分别控制总量、边缘弯折、内部放大和色散，主界面无需新增第二套滑块。
   */
  float titlebarVertical = local.y * 2.0 - 1.0;
  vec2 titlebarWarpPx = vec2(
    (local.x - .5) * 18.0 * uMagnificationStrength,
    (titlebarArc - .50) * 58.0 * uEdgeStrength
      + titlebarVertical * 18.0 * uMagnificationStrength
  ) * uDistortionIntensity;
  titlebarWarpPx += edgeNormal
    * (10.0 + 12.0 * titlebarArc)
    * uEdgeStrength
    * uDistortionIntensity;
  displacementPx = mix(displacementPx, titlebarWarpPx, titlebarSurface);

  /* 指针只在当前玻璃内部产生局部压力与光学形变。 */
  vec2 pointerDeltaPx = (uPointer - uv) * uResolution;
  float pointerRadiusPx = mix(82.0, 148.0, uQuality);
  float pointerLens = exp(
    -dot(pointerDeltaPx, pointerDeltaPx) /
    max(2.0 * pointerRadiusPx * pointerRadiusPx, 1.0)
  );
  displacementPx += safeNormalize(pointerDeltaPx)
    * pointerLens
    * mix(1.4, 7.6, uQuality)
    * uDistortionIntensity;

  /* 极轻微表面起伏绑定在边缘，避免变成廉价的常驻水波纹。 */
  displacementPx += vec2(
    sin((uv.y + uTime * .012) * 27.0),
    cos((uv.x - uTime * .010) * 25.0)
  ) * nonlinearRim * .62 * uQuality * uDistortionIntensity;

  sampleUv -= displacementPx * texel * uRefractionEnabled;
  sampleUv = clamp(sampleUv, texel * 1.5, vec2(1.0) - texel * 1.5);

  /* RGB 色散限制在约 0.25–1.1 像素，作为玻璃边缘物理提示而非彩边特效。 */
  float dispersionPx = mix(.32, 1.42, uQuality)
    * mix(.18, 1.0, nonlinearRim)
    * uRefractionEnabled
    * uDispersionEnabled
    * uDispersionStrength
    * uDistortionIntensity;
  dispersionPx *= mix(1.0, 1.72, titlebarSurface);
  vec2 dispersionUv = edgeNormal * texel * dispersionPx;
  float r = texture(uScene, clamp(sampleUv - dispersionUv, 0.0, 1.0)).r;
  float g = texture(uScene, sampleUv).g;
  float b = texture(uScene, clamp(sampleUv + dispersionUv, 0.0, 1.0)).b;
  vec3 refracted = vec3(r, g, b);

  /* Schlick Fresnel：边缘增强，中央保持清透。 */
  float surfaceSlope = clamp(pow(rim, .66) * .985, 0.0, .985);
  float cosTheta = sqrt(max(1.0 - surfaceSlope * surfaceSlope, .001));
  float fresnel = .035 + .965 * pow(1.0 - cosTheta, 5.0);
  vec2 lightDirection = safeNormalize(vec2(-.58, .82));
  float keyReflection = pow(max(dot(edgeNormal, lightDirection), 0.0), 7.0) * nonlinearRim;
  float pointerIllumination = pointerLens * (.012 + .018 * uQuality);
  float topReflection = pow(clamp(1.0 - local.y, 0.0, 1.0), 6.0) * .055;

  vec3 glass = refracted;
  glass *= 1.0 - nonlinearRim * .028;
  glass += mix(vec3(1.0), vec3(.84, .90, 1.0), .14) * fresnel * .17 * uMaterialVisibility;
  glass += vec3(1.0) * keyReflection * .09 * uMaterialVisibility;
  glass += mix(vec3(1.0), vec3(.82, .90, 1.0), .18) * pointerIllumination * uMaterialVisibility;
  glass += vec3(1.0) * topReflection * uMaterialVisibility;

  outColor = vec4(glass, 1.0);
}`

interface RendererState {
  gl: WebGL2RenderingContext
  sceneProgram: WebGLProgram
  compositeProgram: WebGLProgram
  sceneTexture: WebGLTexture
  backgroundTexture: WebGLTexture
  backgroundSize: [number, number]
  hasBackground: boolean
  backgroundMode: 'default' | 'image' | 'video'
  framebuffer: WebGLFramebuffer
  vertexArray: WebGLVertexArrayObject
  buffer: WebGLBuffer
}

interface QaRect {
  x: number
  y: number
  width: number
  height: number
  radius: number
}

interface QaRegion {
  values: number[]
  changed: number
}

interface GlassQaWindow extends Window {
  __TOKEN_MANAGER_GLASS_QA__?: () => Record<string, unknown>
  __TOKEN_MANAGER_GLASS_SURFACE_AUDIT__?: () => Record<string, unknown>
}

type VideoFrameElement = HTMLVideoElement & {
  requestVideoFrameCallback?: (callback: (now: DOMHighResTimeStamp) => void) => number
  cancelVideoFrameCallback?: (handle: number) => void
}

let state: RendererState | null = null
let animationFrame = 0
let resizeObserver: ResizeObserver | null = null
let mutationObserver: MutationObserver | null = null
let rectsDirty = true
let lastRectRefresh = 0
let startTime = performance.now()
let pointerTarget = { x: .62, y: .82 }
let pointerCurrent = { ...pointerTarget }
const rectData = new Float32Array(MAX_SURFACES * 4)
const radiusData = new Float32Array(MAX_SURFACES)
const surfaceMetaData = new Float32Array(MAX_SURFACES * 2)
let rectCount = 0
let backgroundLoadToken = 0
let videoFrameDirty = false
let videoFrameCallback = 0
let videoPlaybackFailed = false
let videoUploadFailureCount = 0
let lastVisibleMaterialSurfaces: HTMLElement[] = []
let lastTrackedMaterialSurfaces: HTMLElement[] = []
let lastSuppressedNestedSurfaces: HTMLElement[] = []

function compileShader(gl: WebGL2RenderingContext, type: number, source: string) {
  const shader = gl.createShader(type)
  if (!shader) throw new Error('无法创建液态玻璃着色器')
  gl.shaderSource(shader, source)
  gl.compileShader(shader)
  if (!gl.getShaderParameter(shader, gl.COMPILE_STATUS)) {
    const log = gl.getShaderInfoLog(shader) || '未知着色器错误'
    gl.deleteShader(shader)
    throw new Error(log)
  }
  return shader
}

function createProgram(gl: WebGL2RenderingContext, fragmentSource: string) {
  const program = gl.createProgram()
  if (!program) throw new Error('无法创建液态玻璃渲染程序')
  const vertex = compileShader(gl, gl.VERTEX_SHADER, VERTEX_SHADER)
  const fragment = compileShader(gl, gl.FRAGMENT_SHADER, fragmentSource)
  gl.attachShader(program, vertex)
  gl.attachShader(program, fragment)
  gl.linkProgram(program)
  gl.deleteShader(vertex)
  gl.deleteShader(fragment)
  if (!gl.getProgramParameter(program, gl.LINK_STATUS)) {
    const log = gl.getProgramInfoLog(program) || '未知链接错误'
    gl.deleteProgram(program)
    throw new Error(log)
  }
  return program
}

function cssColorToRgb(value: string, fallback: [number, number, number]) {
  const normalized = value.trim()
  const match = normalized.match(/^#([\da-f]{6})$/i)
  if (!match) return fallback
  const integer = Number.parseInt(match[1], 16)
  return [
    ((integer >> 16) & 255) / 255,
    ((integer >> 8) & 255) / 255,
    (integer & 255) / 255,
  ] as [number, number, number]
}

async function loadBackgroundTexture(url: string) {
  const renderer = state
  const token = ++backgroundLoadToken
  if (!renderer) return
  if (!url) {
    renderer.hasBackground = false
    renderer.backgroundSize = [1, 1]
    renderer.backgroundMode = 'default'
    document.documentElement.dataset.glassBackground = 'default'
    delete document.documentElement.dataset.glassMedia
    startRenderer()
    return
  }
  try {
    const image = new Image()
    image.decoding = 'async'
    image.src = url
    await image.decode()
    if (token !== backgroundLoadToken || !state) return
    const { gl } = state
    gl.activeTexture(gl.TEXTURE1)
    gl.bindTexture(gl.TEXTURE_2D, state.backgroundTexture)
    gl.pixelStorei(gl.UNPACK_FLIP_Y_WEBGL, true)
    gl.texImage2D(gl.TEXTURE_2D, 0, gl.RGBA, gl.RGBA, gl.UNSIGNED_BYTE, image)
    gl.pixelStorei(gl.UNPACK_FLIP_Y_WEBGL, false)
    state.backgroundSize = [Math.max(1, image.naturalWidth), Math.max(1, image.naturalHeight)]
    state.hasBackground = true
    state.backgroundMode = 'image'
    document.documentElement.dataset.glassBackground = 'custom'
    document.documentElement.dataset.glassMedia = 'image'
    startRenderer()
  } catch (error) {
    console.warn('液态玻璃背景图片解码失败，已保留默认空间背景：', error)
    if (token === backgroundLoadToken && state) {
      state.hasBackground = false
      state.backgroundSize = [1, 1]
      state.backgroundMode = 'default'
      document.documentElement.dataset.glassBackground = 'default'
      delete document.documentElement.dataset.glassMedia
      startRenderer()
    }
  }
}

function cancelVideoFrameTracking() {
  const video = videoSource.value as VideoFrameElement | null
  if (videoFrameCallback && video?.cancelVideoFrameCallback) {
    video.cancelVideoFrameCallback(videoFrameCallback)
  }
  videoFrameCallback = 0
}

function scheduleVideoFrameTracking() {
  cancelVideoFrameTracking()
  const video = videoSource.value as VideoFrameElement | null
  if (!video || !props.videoUrl || !video.requestVideoFrameCallback || videoPlaybackFailed) return
  videoFrameCallback = video.requestVideoFrameCallback(() => {
    videoFrameCallback = 0
    videoFrameDirty = true
    if (!motionEnabled.value) startRenderer()
    scheduleVideoFrameTracking()
  })
}

function videoErrorMessage() {
  const code = videoSource.value?.error?.code
  if (code === MediaError.MEDIA_ERR_SRC_NOT_SUPPORTED) {
    return '视频壁纸无法播放：当前 WebView2 不支持此视频编码。请改用 H.264 MP4 或 VP9 WebM，或清除视频壁纸。'
  }
  if (code === MediaError.MEDIA_ERR_DECODE) {
    return '视频壁纸解码失败，文件可能损坏或编码不受支持。请更换视频或清除视频壁纸。'
  }
  return '视频壁纸播放失败。请确认文件仍在原位置，或更换视频后重试。'
}

function fallBackFromVideo(message: string) {
  if (videoPlaybackFailed) return
  videoPlaybackFailed = true
  cancelVideoFrameTracking()
  emit('videoError', message)
  void loadBackgroundTexture(props.backgroundUrl)
}

function uploadVideoFrame() {
  const renderer = state
  const video = videoSource.value
  if (!renderer || renderer.backgroundMode !== 'video' || !video || video.readyState < HTMLMediaElement.HAVE_CURRENT_DATA) return
  const supportsFrameCallback = Boolean((video as VideoFrameElement).requestVideoFrameCallback)
  if (supportsFrameCallback && !videoFrameDirty) return
  const { gl } = renderer
  try {
    gl.activeTexture(gl.TEXTURE1)
    gl.bindTexture(gl.TEXTURE_2D, renderer.backgroundTexture)
    gl.pixelStorei(gl.UNPACK_FLIP_Y_WEBGL, true)
    // 每一帧直接更新 WebGL 环境纹理；液态玻璃的折射、色散会真实作用于动态画面。
    gl.texImage2D(gl.TEXTURE_2D, 0, gl.RGBA, gl.RGBA, gl.UNSIGNED_BYTE, video)
    const uploadError = gl.getError()
    if (uploadError !== gl.NO_ERROR) {
      throw new Error(`视频纹理上传失败：0x${uploadError.toString(16)}`)
    }
    renderer.backgroundSize = [Math.max(1, video.videoWidth), Math.max(1, video.videoHeight)]
    renderer.hasBackground = true
    videoFrameDirty = false
    videoUploadFailureCount = 0
  } catch (error) {
    videoFrameDirty = true
    videoUploadFailureCount += 1
    const detail = error instanceof DOMException
      ? `${error.name}: ${error.message}`
      : String(error)
    console.warn(`视频帧上传到液态玻璃纹理失败（第 ${videoUploadFailureCount} 次）：`, detail)

    // WebView2 刚切换解码表面时偶尔会有一帧尚不可上传；允许后续视频帧自动重试。
    // 跨源污染无法靠重试恢复，连续三帧失败也说明当前编码无法作为 WebGL 纹理使用。
    const isSecurityError = error instanceof DOMException && error.name === 'SecurityError'
    if (isSecurityError || videoUploadFailureCount >= 3) {
      fallBackFromVideo(
        isSecurityError
          ? '本地视频没有通过 WebGL 跨源校验，已回退到图片或默认背景。请重新选择视频。'
          : `视频壁纸无法进入液态玻璃渲染器：${detail}`,
      )
    }
  } finally {
    // 即使纹理上传抛错也必须恢复像素方向，避免后续图片降级路径被上下翻转。
    gl.pixelStorei(gl.UNPACK_FLIP_Y_WEBGL, false)
  }
}

function handleVideoLoaded() {
  const renderer = state
  const video = videoSource.value
  if (!renderer || !video || !props.videoUrl || video.videoWidth < 1 || video.videoHeight < 1) return
  backgroundLoadToken += 1
  videoPlaybackFailed = false
  videoUploadFailureCount = 0
  renderer.backgroundMode = 'video'
  renderer.backgroundSize = [video.videoWidth, video.videoHeight]
  renderer.hasBackground = true
  videoFrameDirty = true
  document.documentElement.dataset.glassBackground = 'custom'
  document.documentElement.dataset.glassMedia = 'video'
  void video.play().catch(() => {
    fallBackFromVideo('视频壁纸自动播放被系统阻止。请更换视频或清除视频壁纸后重试。')
  })
  scheduleVideoFrameTracking()
  startRenderer()
}

function handleVideoPlaybackError() {
  fallBackFromVideo(videoErrorMessage())
}

function updateTextureSize() {
  const node = canvas.value
  const renderer = state
  if (!node || !renderer) return
  const ratioLimit = props.quality === 'high' ? 2 : 1
  const ratio = Math.min(ratioLimit, window.devicePixelRatio || 1)
  const width = Math.max(1, Math.round(node.clientWidth * ratio))
  const height = Math.max(1, Math.round(node.clientHeight * ratio))
  if (node.width === width && node.height === height) return
  node.width = width
  node.height = height
  renderer.gl.bindTexture(renderer.gl.TEXTURE_2D, renderer.sceneTexture)
  renderer.gl.texImage2D(renderer.gl.TEXTURE_2D, 0, renderer.gl.RGBA8, width, height, 0, renderer.gl.RGBA, renderer.gl.UNSIGNED_BYTE, null)
  renderer.gl.bindFramebuffer(renderer.gl.FRAMEBUFFER, renderer.framebuffer)
  const framebufferStatus = renderer.gl.checkFramebufferStatus(renderer.gl.FRAMEBUFFER)
  renderer.gl.bindFramebuffer(renderer.gl.FRAMEBUFFER, null)
  if (framebufferStatus !== renderer.gl.FRAMEBUFFER_COMPLETE) {
    throw new Error(`液态玻璃真实尺寸缓冲不完整：0x${framebufferStatus.toString(16)}`)
  }
  rectsDirty = true
}

function updateSurfaceRects() {
  const node = canvas.value
  if (!node) return
  lastTrackedMaterialSurfaces.forEach(surface => delete surface.dataset.liquidOpticalSurface)
  lastSuppressedNestedSurfaces.forEach(surface => delete surface.dataset.liquidNestedSuppressed)
  rectData.fill(0)
  radiusData.fill(0)
  surfaceMetaData.fill(0)
  const maximum = MAX_SURFACES
  const isVisible = (surface: HTMLElement) => {
    const box = surface.getBoundingClientRect()
    return box.width > 24 && box.height > 18 && box.right > 0 && box.bottom > 0
      && box.left < window.innerWidth && box.top < window.innerHeight
  }
  const declared = [...document.querySelectorAll<HTMLElement>(SURFACE_SELECTOR)]
    .filter(surface => !surface.matches(GROUP_ONLY_SELECTOR) && !surface.closest('[data-liquid-ignore]'))
  const declaredSet = new Set(declared)
  const automaticRaw = [...document.querySelectorAll<HTMLElement>('main :is(section,article,aside,header,nav,footer,div)')]
    .filter(surface => {
      if (surface.closest('.liquid-glass-environment,[data-liquid-ignore]') || surface.matches('.content,.page-panel,.analytics-suite,.grid,.metrics')) return false
      // 自动识别只补足没有显式材质边界的孤立组件，绝不包裹或钻入已声明玻璃。
      if (surface.closest(SURFACE_SELECTOR) || surface.querySelector(SURFACE_SELECTOR)) return false
      const style = getComputedStyle(surface)
      const radius = Number.parseFloat(style.borderTopLeftRadius) || 0
      const hasPaint = style.backgroundImage !== 'none'
        || !['transparent', 'rgba(0, 0, 0, 0)'].includes(style.backgroundColor)
        || Number.parseFloat(style.borderTopWidth) > 0
      return radius >= 9 && hasPaint
    })
  // 自动候选之间保留最内层的实际组件，避免把整个布局容器误判为一块巨型玻璃。
  const automatic = automaticRaw.filter(surface => !automaticRaw.some(other => other !== surface && surface.contains(other)))
  const rawCandidates = [...new Set([...declared, ...automatic])].filter(isVisible)
  const rawCandidateSet = new Set(rawCandidates)
  lastSuppressedNestedSurfaces = rawCandidates.filter(surface => {
    const ancestor = surface.parentElement?.closest<HTMLElement>(SURFACE_SELECTOR)
    return Boolean(ancestor && rawCandidateSet.has(ancestor) && declaredSet.has(ancestor))
  })
  const suppressedSet = new Set(lastSuppressedNestedSurfaces)
  const candidates = rawCandidates
    .filter(surface => !suppressedSet.has(surface))
    .sort((a, b) => {
      const boxA = a.getBoundingClientRect()
      const boxB = b.getBoundingClientRect()
      const priority = (surface: HTMLElement) => {
        if (surface.dataset.liquidSurface === 'primary'
          || surface.matches('.theme-liquid-glass .app-titlebar,.theme-liquid-glass.shell > aside,.theme-liquid-glass .content > header,.theme-liquid-glass.floating-shell')) return 0
        if (surface.hasAttribute('data-liquid-surface')) return 1
        if (surface.matches(SURFACE_SELECTOR)) return 2
        return 3
      }
      const priorityDifference = priority(a) - priority(b)
      // 去嵌套后，同级优先保留较小的独立组件，充分利用固定 GPU 表面预算。
      return priorityDifference || boxA.width * boxA.height - boxB.width * boxB.height
    })
  lastVisibleMaterialSurfaces = candidates
  lastTrackedMaterialSurfaces = candidates.slice(0, maximum)
  lastTrackedMaterialSurfaces.forEach(surface => { surface.dataset.liquidOpticalSurface = 'true' })
  lastSuppressedNestedSurfaces.forEach(surface => { surface.dataset.liquidNestedSuppressed = 'true' })

  rectCount = lastTrackedMaterialSurfaces.length
  lastTrackedMaterialSurfaces.forEach((surface, index) => {
    const box = surface.getBoundingClientRect()
    const style = getComputedStyle(surface)
    rectData[index * 4] = box.left / Math.max(1, window.innerWidth)
    rectData[index * 4 + 1] = (window.innerHeight - box.bottom) / Math.max(1, window.innerHeight)
    rectData[index * 4 + 2] = box.width / Math.max(1, window.innerWidth)
    rectData[index * 4 + 3] = box.height / Math.max(1, window.innerHeight)
    radiusData[index] = Number.parseFloat(style.borderTopLeftRadius) * (node.width / Math.max(1, node.clientWidth)) || 16
    surfaceMetaData[index * 2] = radiusData[index]
    surfaceMetaData[index * 2 + 1] = surface.matches('.theme-liquid-glass .app-titlebar') ? 1 : 0
  })
  rectsDirty = false
  lastRectRefresh = performance.now()
}

function describeMaterialSurface(surface: HTMLElement) {
  const id = surface.id ? `#${surface.id}` : ''
  const classes = [...surface.classList].filter(name => !name.startsWith('tm-')).slice(0, 3).map(name => `.${name}`).join('')
  return `${surface.tagName.toLowerCase()}${id}${classes}`
}

/** 自动化验收入口：报告当前视口全部玻璃候选、实际 GPU 折射表面与遗漏项。 */
function runSurfaceAudit() {
  updateSurfaceRects()
  const tracked = new Set(lastTrackedMaterialSurfaces)
  const missed = lastVisibleMaterialSurfaces.filter(surface => !tracked.has(surface))
  const overlaps = lastTrackedMaterialSurfaces.flatMap((surface, index) => lastTrackedMaterialSurfaces
    .slice(index + 1)
    .filter(other => surface.contains(other) || other.contains(surface))
    .map(other => `${describeMaterialSurface(surface)} ↔ ${describeMaterialSurface(other)}`))
  return {
    available: Boolean(state),
    quality: props.quality,
    budget: MAX_SURFACES,
    visible: lastVisibleMaterialSurfaces.length,
    tracked: lastTrackedMaterialSurfaces.length,
    missed: missed.length,
    missedSurfaces: missed.slice(0, 12).map(describeMaterialSurface),
    suppressedNested: lastSuppressedNestedSurfaces.length,
    overlapping: overlaps.length,
    overlappingSurfaces: overlaps.slice(0, 12),
    distortion: { ...props.distortion },
    titlebar: {
      tracked: lastTrackedMaterialSurfaces.some(surface => surface.matches('.theme-liquid-glass .app-titlebar')),
      profile: 'continuous-strong-arc',
    },
    passed: missed.length === 0 && overlaps.length === 0,
  }
}

function roundedRectDistancePixels(x: number, y: number, rect: QaRect) {
  const radius = Math.max(0, Math.min(rect.radius, rect.width / 2, rect.height / 2))
  const px = Math.abs(x - (rect.x + rect.width / 2))
  const py = Math.abs(y - (rect.y + rect.height / 2))
  const qx = px - Math.max(rect.width / 2 - radius, 0)
  const qy = py - Math.max(rect.height / 2 - radius, 0)
  return Math.hypot(Math.max(qx, 0), Math.max(qy, 0)) + Math.min(Math.max(qx, qy), 0) - radius
}

function summarizeQaRegion(region: QaRegion) {
  if (!region.values.length) return { count: 0, rms: 0, changedRate: 0, p95: 0 }
  let squareSum = 0
  for (const value of region.values) squareSum += value * value
  region.values.sort((a, b) => a - b)
  return {
    count: region.values.length,
    rms: Math.sqrt(squareSum / region.values.length),
    changedRate: region.changed / region.values.length,
    p95: region.values[Math.floor((region.values.length - 1) * .95)],
  }
}

function restoreDatasetValue(key: string, value: string | undefined) {
  if (value === undefined) delete document.documentElement.dataset[key]
  else document.documentElement.dataset[key] = value
}

/**
 * 直接读取 WebGL 默认帧缓冲，不经过 DOM 截图或浏览器二次合成。
 * 开关对照仅改变采样坐标位移和色散，场景、时间、指针与 Fresnel 层完全一致。
 */
function runRefractionQa() {
  const renderer = state
  const node = canvas.value
  if (!renderer || !node) return { available: false, reason: 'WebGL2 渲染器尚未就绪' }

  const titlebarQa = document.documentElement.dataset.glassQaTarget === 'titlebar'
  const candidates = Array.from(document.querySelectorAll<HTMLElement>(titlebarQa
    ? '.theme-liquid-glass .app-titlebar'
    : '.tm-spotlight-active, .deepseek-metrics > .card, .deepseek-monitor > .balance-chart, .model-kpis > .card, .provider-overview, .visual-card'))
  const target = candidates.find(element => {
    const rect = element.getBoundingClientRect()
    return rect.width > 0 && rect.height > 0 && rect.bottom > 0 && rect.top < window.innerHeight
  }) || candidates[0]
  if (!target) return { available: false, reason: '当前视口没有可验证的玻璃表面' }

  const root = document.documentElement
  const previous = {
    glassFreeze: root.dataset.glassFreeze,
    glassRefraction: root.dataset.glassRefraction,
    glassDispersion: root.dataset.glassDispersion,
  }

  stopRenderer()
  try {
    root.dataset.glassFreeze = '1'
    root.dataset.glassDispersion = 'off'
    rectsDirty = true
    updateTextureSize()
    updateSurfaceRects()

    const { gl } = renderer
    const width = gl.drawingBufferWidth
    const height = gl.drawingBufferHeight
    const targetBox = target.getBoundingClientRect()
    const scaleX = width / Math.max(1, window.innerWidth)
    const scaleY = height / Math.max(1, window.innerHeight)
    const targetStyle = getComputedStyle(target)
    const targetRect: QaRect = {
      x: targetBox.left * scaleX,
      y: (window.innerHeight - targetBox.bottom) * scaleY,
      width: targetBox.width * scaleX,
      height: targetBox.height * scaleY,
      radius: (Number.parseFloat(targetStyle.borderTopLeftRadius) || 16) * scaleX,
    }
    const surfaceRects: QaRect[] = Array.from({ length: rectCount }, (_, index) => ({
      x: rectData[index * 4] * width,
      y: rectData[index * 4 + 1] * height,
      width: rectData[index * 4 + 2] * width,
      height: rectData[index * 4 + 3] * height,
      radius: radiusData[index],
    }))

    const capturePixels = (enabled: boolean) => {
      root.dataset.glassRefraction = enabled ? 'on' : 'off'
      render(performance.now())
      gl.finish()
      const pixels = new Uint8Array(width * height * 4)
      gl.readPixels(0, 0, width, height, gl.RGBA, gl.UNSIGNED_BYTE, pixels)
      return pixels
    }

    const off = capturePixels(false)
    const on = capturePixels(true)
    const regions: Record<'inside' | 'edge' | 'core' | 'outside', QaRegion> = {
      inside: { values: [], changed: 0 },
      edge: { values: [], changed: 0 },
      core: { values: [], changed: 0 },
      outside: { values: [], changed: 0 },
    }

    for (let y = 0; y < height; y += 1) {
      for (let x = 0; x < width; x += 1) {
        const targetDistance = roundedRectDistancePixels(x + .5, y + .5, targetRect)
        const insideTarget = targetDistance <= (titlebarQa ? -2 : -4)
        const inEdge = titlebarQa
          ? targetDistance <= -2 && targetDistance >= -6
          : targetDistance <= -3 && targetDistance >= -18
        const inCore = targetDistance <= (titlebarQa ? -8 : -24)
        let outsideAllSurfaces = !insideTarget && !inEdge && !inCore
        if (outsideAllSurfaces) {
          for (const surface of surfaceRects) {
            if (roundedRectDistancePixels(x + .5, y + .5, surface) <= 8) {
              outsideAllSurfaces = false
              break
            }
          }
        }

        const index = (y * width + x) * 4
        const red = Math.abs(off[index] - on[index])
        const green = Math.abs(off[index + 1] - on[index + 1])
        const blue = Math.abs(off[index + 2] - on[index + 2])
        const delta = Math.sqrt((red * red + green * green + blue * blue) / 3)
        const changed = Math.max(red, green, blue) >= 2

        if (insideTarget) {
          regions.inside.values.push(delta)
          if (changed) regions.inside.changed += 1
        }
        if (inEdge) {
          regions.edge.values.push(delta)
          if (changed) regions.edge.changed += 1
        } else if (inCore) {
          regions.core.values.push(delta)
          if (changed) regions.core.changed += 1
        }
        if (outsideAllSurfaces) {
          regions.outside.values.push(delta)
          if (changed) regions.outside.changed += 1
        }
      }
    }

    const inside = summarizeQaRegion(regions.inside)
    const edge = summarizeQaRegion(regions.edge)
    const core = summarizeQaRegion(regions.core)
    const outside = summarizeQaRegion(regions.outside)
    const edgeCoreRatio = core.rms === 0 ? null : edge.rms / core.rms
    const insideOutsideRatio = outside.rms === 0 ? null : inside.rms / outside.rms
    const thresholds = {
      edgeRms: edge.rms >= 3,
      edgeChangedRate: edge.changedRate >= .30,
      edgeP95: edge.p95 >= 8,
      coreRms: core.rms >= 1.5,
      outsideRms: outside.rms <= .01,
      outsideChangedRate: outside.changedRate === 0,
      // 普通卡片强调厚边；标题栏要求整条等强圆弧，因此边缘与中央应接近而不是突然衰减。
      edgeCoreRatio: edgeCoreRatio === null || (titlebarQa
        ? edgeCoreRatio >= .82 && edgeCoreRatio <= 1.30
        : edgeCoreRatio >= 1.25),
    }
    return {
      available: true,
      targetKind: titlebarQa ? 'titlebar-strong-arc' : 'standard-surface',
      renderer: gl.getParameter(gl.RENDERER),
      viewport: { width, height },
      target: targetRect,
      inside,
      edge,
      core,
      outside,
      edgeCoreRatio,
      insideOutsideRatio,
      thresholds,
      passed: Object.values(thresholds).every(Boolean),
    }
  } finally {
    restoreDatasetValue('glassFreeze', previous.glassFreeze)
    restoreDatasetValue('glassRefraction', previous.glassRefraction)
    restoreDatasetValue('glassDispersion', previous.glassDispersion)
    startRenderer()
  }
}

function setCommonUniforms(gl: WebGL2RenderingContext, program: WebGLProgram, elapsed: number) {
  gl.uniform2f(gl.getUniformLocation(program, 'uResolution'), gl.drawingBufferWidth, gl.drawingBufferHeight)
  gl.uniform2f(gl.getUniformLocation(program, 'uPointer'), pointerCurrent.x, pointerCurrent.y)
  gl.uniform1f(gl.getUniformLocation(program, 'uTime'), elapsed)
}

function render(now: number, scheduleNext = true) {
  const renderer = state
  if (!renderer) return
  if (renderer.gl.isContextLost()) throw new Error('WebGL 上下文已丢失')
  updateTextureSize()
  if (rectsDirty || now - lastRectRefresh > (props.quality === 'high' ? 180 : 420)) updateSurfaceRects()

  const { gl } = renderer
  uploadVideoFrame()
  const frozen = document.documentElement.dataset.glassFreeze === '1'
  const refractionEnabled = document.documentElement.dataset.glassRefraction !== 'off'
  const dispersionEnabled = document.documentElement.dataset.glassDispersion !== 'off'
  const elapsed = frozen ? 12.5 : (now - startTime) / 1000
  const damping = props.quality === 'high' ? .085 : .16
  if (frozen) {
    pointerCurrent = { ...pointerTarget }
  } else {
    pointerCurrent.x += (pointerTarget.x - pointerCurrent.x) * damping
    pointerCurrent.y += (pointerTarget.y - pointerCurrent.y) * damping
  }
  const rootStyle = getComputedStyle(document.documentElement)
  const accent = cssColorToRgb(rootStyle.getPropertyValue('--tm-accent'), [.47, .66, 1])
  const glow = cssColorToRgb(rootStyle.getPropertyValue('--tm-glow'), [.61, .47, 1])
  const node = canvas.value
  if (node) {
    node.dataset.refraction = refractionEnabled ? 'on' : 'off'
    node.dataset.dispersion = dispersionEnabled ? 'on' : 'off'
    node.dataset.freeze = frozen ? 'on' : 'off'
  }

  gl.bindVertexArray(renderer.vertexArray)
  gl.viewport(0, 0, gl.drawingBufferWidth, gl.drawingBufferHeight)

  // 第一通道：在 GPU 离屏纹理中生成可被折射的空间环境光场。
  gl.bindFramebuffer(gl.FRAMEBUFFER, renderer.framebuffer)
  gl.useProgram(renderer.sceneProgram)
  setCommonUniforms(gl, renderer.sceneProgram, elapsed)
  gl.uniform1f(gl.getUniformLocation(renderer.sceneProgram, 'uMotion'), motionEnabled.value && !frozen ? 1 : 0)
  gl.uniform3fv(gl.getUniformLocation(renderer.sceneProgram, 'uAccent'), accent)
  gl.uniform3fv(gl.getUniformLocation(renderer.sceneProgram, 'uGlow'), glow)
  gl.uniform1f(gl.getUniformLocation(renderer.sceneProgram, 'uClearTone'), liquidTone.value === 'clear' ? 1 : 0)
  gl.activeTexture(gl.TEXTURE1)
  gl.bindTexture(gl.TEXTURE_2D, renderer.backgroundTexture)
  gl.uniform1i(gl.getUniformLocation(renderer.sceneProgram, 'uBackground'), 1)
  gl.uniform1f(gl.getUniformLocation(renderer.sceneProgram, 'uHasBackground'), renderer.hasBackground ? 1 : 0)
  gl.uniform2f(
    gl.getUniformLocation(renderer.sceneProgram, 'uBackgroundSize'),
    renderer.backgroundSize[0],
    renderer.backgroundSize[1],
  )
  gl.drawArrays(gl.TRIANGLES, 0, 6)

  // 第二通道：按真实 DOM 卡片轮廓执行透镜位移、色散、厚度与 Fresnel 边缘合成。
  gl.bindFramebuffer(gl.FRAMEBUFFER, null)
  gl.useProgram(renderer.compositeProgram)
  setCommonUniforms(gl, renderer.compositeProgram, elapsed)
  gl.activeTexture(gl.TEXTURE0)
  gl.bindTexture(gl.TEXTURE_2D, renderer.sceneTexture)
  gl.uniform1i(gl.getUniformLocation(renderer.compositeProgram, 'uScene'), 0)
  gl.uniform1f(gl.getUniformLocation(renderer.compositeProgram, 'uQuality'), props.quality === 'high' ? 1 : .32)
  gl.uniform1f(gl.getUniformLocation(renderer.compositeProgram, 'uRefractionEnabled'), refractionEnabled ? 1 : 0)
  gl.uniform1f(gl.getUniformLocation(renderer.compositeProgram, 'uDispersionEnabled'), dispersionEnabled ? 1 : 0)
  gl.uniform1f(gl.getUniformLocation(renderer.compositeProgram, 'uDistortionIntensity'), props.distortion.intensity)
  gl.uniform1f(gl.getUniformLocation(renderer.compositeProgram, 'uEdgeStrength'), props.distortion.edgeBend)
  gl.uniform1f(gl.getUniformLocation(renderer.compositeProgram, 'uMagnificationStrength'), props.distortion.magnification)
  gl.uniform1f(gl.getUniformLocation(renderer.compositeProgram, 'uDispersionStrength'), props.distortion.dispersion)
  gl.uniform1f(
    gl.getUniformLocation(renderer.compositeProgram, 'uMaterialVisibility'),
    .34 + (1 - liquidTransparency.value / 100) * .66,
  )
  gl.uniform1i(gl.getUniformLocation(renderer.compositeProgram, 'uRectCount'), rectCount)
  gl.uniform4fv(gl.getUniformLocation(renderer.compositeProgram, 'uRects[0]'), rectData)
  gl.uniform2fv(gl.getUniformLocation(renderer.compositeProgram, 'uSurfaceMeta[0]'), surfaceMetaData)
  gl.drawArrays(gl.TRIANGLES, 0, 6)

  /* QA 冻结时只绘制一帧，避免截图恰好撞上 GPU 正在合成下一帧。 */
  const videoNeedsContinuousRaf = renderer.backgroundMode === 'video'
    && Boolean(props.videoUrl)
    && !Boolean((videoSource.value as VideoFrameElement | null)?.requestVideoFrameCallback)
  if (scheduleNext && (motionEnabled.value || videoNeedsContinuousRaf) && !frozen) {
    animationFrame = requestAnimationFrame(renderFrame)
  }
}

function stopRenderer() {
  cancelAnimationFrame(animationFrame)
  animationFrame = 0
}

function releaseRendererState() {
  const renderer = state
  state = null
  if (!renderer) return
  try {
    const { gl } = renderer
    gl.deleteProgram(renderer.sceneProgram)
    gl.deleteProgram(renderer.compositeProgram)
    gl.deleteTexture(renderer.sceneTexture)
    gl.deleteTexture(renderer.backgroundTexture)
    gl.deleteFramebuffer(renderer.framebuffer)
    gl.deleteVertexArray(renderer.vertexArray)
    gl.deleteBuffer(renderer.buffer)
  } catch {
    // 上下文丢失时资源已经由浏览器回收，无需阻塞 CSS 安全降级。
  }
}

function enterCssSafeMode(reason: string) {
  stopRenderer()
  cancelVideoFrameTracking()
  releaseRendererState()
  document.documentElement.classList.remove('glass-webgl-active', 'glass-webgl-pending')
  document.documentElement.classList.add('glass-webgl-fallback')
  document.documentElement.dataset.glassRenderer = reason
}

function renderFrame(now: number) {
  try {
    render(now)
  } catch (error) {
    console.error('LiquidGlassRenderer 运行时失败，已切换 CSS 安全折射：', error)
    enterCssSafeMode('css-webgl-render-failed')
  }
}

function startRenderer() {
  stopRenderer()
  if (!state) return
  animationFrame = requestAnimationFrame(renderFrame)
}

function verifyFirstFrame(gl: WebGL2RenderingContext) {
  gl.finish()
  if (gl.isContextLost()) throw new Error('WebGL 上下文在首帧验证时丢失')

  const error = gl.getError()
  if (error !== gl.NO_ERROR) throw new Error(`WebGL 首帧错误：0x${error.toString(16)}`)

  const width = Math.max(1, gl.drawingBufferWidth)
  const height = Math.max(1, gl.drawingBufferHeight)
  const sample = new Uint8Array(4)
  const points = [
    [Math.floor(width * .2), Math.floor(height * .2)],
    [Math.floor(width * .5), Math.floor(height * .5)],
    [Math.floor(width * .8), Math.floor(height * .8)],
  ]
  let brightest = 0
  let opaqueSamples = 0
  for (const [x, y] of points) {
    gl.readPixels(x, y, 1, 1, gl.RGBA, gl.UNSIGNED_BYTE, sample)
    brightest = Math.max(brightest, sample[0], sample[1], sample[2])
    if (sample[3] > 0) opaqueSamples += 1
  }
  const readError = gl.getError()
  if (readError !== gl.NO_ERROR) throw new Error(`WebGL 首帧像素读取失败：0x${readError.toString(16)}`)
  if (opaqueSamples === 0 || brightest < 4) throw new Error('WebGL 首帧没有可见像素')
}

function initializeRenderer() {
  document.documentElement.classList.remove('glass-webgl-active', 'glass-webgl-fallback')
  document.documentElement.classList.add('glass-webgl-pending')
  document.documentElement.dataset.glassRenderer = 'webgl2-pending'
  const node = canvas.value
  if (!node) {
    enterCssSafeMode('css-canvas-unavailable')
    return
  }
  const gl = node.getContext('webgl2', {
    alpha: true,
    antialias: false,
    depth: false,
    stencil: false,
    premultipliedAlpha: true,
    powerPreference: 'default',
    failIfMajorPerformanceCaveat: true,
  })
  if (!gl) {
    enterCssSafeMode('css-webgl-unavailable')
    return
  }

  try {
    const sceneProgram = createProgram(gl, ENVIRONMENT_SHADER)
    const compositeProgram = createProgram(gl, COMPOSITE_SHADER)
    const sceneTexture = gl.createTexture()
    const backgroundTexture = gl.createTexture()
    const framebuffer = gl.createFramebuffer()
    const vertexArray = gl.createVertexArray()
    const buffer = gl.createBuffer()
    if (!sceneTexture || !backgroundTexture || !framebuffer || !vertexArray || !buffer) throw new Error('液态玻璃 GPU 资源创建失败')

    gl.bindTexture(gl.TEXTURE_2D, sceneTexture)
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MIN_FILTER, gl.LINEAR)
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MAG_FILTER, gl.LINEAR)
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_S, gl.CLAMP_TO_EDGE)
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_T, gl.CLAMP_TO_EDGE)
    gl.texImage2D(gl.TEXTURE_2D, 0, gl.RGBA8, 1, 1, 0, gl.RGBA, gl.UNSIGNED_BYTE, null)
    gl.activeTexture(gl.TEXTURE1)
    gl.bindTexture(gl.TEXTURE_2D, backgroundTexture)
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MIN_FILTER, gl.LINEAR)
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MAG_FILTER, gl.LINEAR)
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_S, gl.CLAMP_TO_EDGE)
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_T, gl.CLAMP_TO_EDGE)
    gl.texImage2D(
      gl.TEXTURE_2D,
      0,
      gl.RGBA,
      1,
      1,
      0,
      gl.RGBA,
      gl.UNSIGNED_BYTE,
      new Uint8Array([7, 10, 18, 255]),
    )
    gl.activeTexture(gl.TEXTURE0)
    gl.bindTexture(gl.TEXTURE_2D, sceneTexture)
    gl.bindFramebuffer(gl.FRAMEBUFFER, framebuffer)
    gl.framebufferTexture2D(gl.FRAMEBUFFER, gl.COLOR_ATTACHMENT0, gl.TEXTURE_2D, sceneTexture, 0)
    const framebufferStatus = gl.checkFramebufferStatus(gl.FRAMEBUFFER)
    if (framebufferStatus !== gl.FRAMEBUFFER_COMPLETE) {
      throw new Error(`液态玻璃离屏缓冲创建失败：0x${framebufferStatus.toString(16)}`)
    }

    gl.bindVertexArray(vertexArray)
    gl.bindBuffer(gl.ARRAY_BUFFER, buffer)
    gl.bufferData(gl.ARRAY_BUFFER, new Float32Array([
      -1, -1, 1, -1, -1, 1,
      -1, 1, 1, -1, 1, 1,
    ]), gl.STATIC_DRAW)
    const positionLocation = gl.getAttribLocation(sceneProgram, 'aPosition')
    gl.enableVertexAttribArray(positionLocation)
    gl.vertexAttribPointer(positionLocation, 2, gl.FLOAT, false, 0, 0)
    const compositePosition = gl.getAttribLocation(compositeProgram, 'aPosition')
    if (compositePosition !== positionLocation) {
      gl.enableVertexAttribArray(compositePosition)
      gl.vertexAttribPointer(compositePosition, 2, gl.FLOAT, false, 0, 0)
    }

    state = {
      gl,
      sceneProgram,
      compositeProgram,
      sceneTexture,
      backgroundTexture,
      backgroundSize: [1, 1],
      hasBackground: false,
      backgroundMode: 'default',
      framebuffer,
      vertexArray,
      buffer,
    }
    updateTextureSize()
    updateSurfaceRects()
    const setupError = gl.getError()
    if (setupError !== gl.NO_ERROR) throw new Error(`WebGL 初始化错误：0x${setupError.toString(16)}`)
    render(performance.now(), false)
    verifyFirstFrame(gl)
    document.documentElement.classList.remove('glass-webgl-pending', 'glass-webgl-fallback')
    document.documentElement.classList.add('glass-webgl-active')
    document.documentElement.dataset.glassRenderer = 'webgl2'
    if (props.videoUrl && (videoSource.value?.readyState ?? 0) >= HTMLMediaElement.HAVE_CURRENT_DATA) {
      handleVideoLoaded()
    } else {
      void loadBackgroundTexture(props.backgroundUrl)
    }
    startRenderer()
  } catch (error) {
    console.error('LiquidGlassRenderer 初始化失败，已切换 CSS 降级模式：', error)
    enterCssSafeMode('css-webgl-initialization-failed')
  }
}

function updatePointer(event: PointerEvent) {
  pointerTarget.x = event.clientX / Math.max(1, window.innerWidth)
  pointerTarget.y = 1 - event.clientY / Math.max(1, window.innerHeight)
}

function markRectsDirty() {
  rectsDirty = true
}

function renderOnce() {
  startRenderer()
}

function handleContextLost(event: Event) {
  event.preventDefault()
  cancelVideoFrameTracking()
  enterCssSafeMode('css-webgl-context-lost')
}

function handleContextRestored() {
  initializeRenderer()
}

watch(() => props.quality, () => {
  rectsDirty = true
  startRenderer()
})
watch(() => props.distortion, () => startRenderer(), { deep: true })
watch(() => props.backgroundUrl, value => {
  if (!props.videoUrl || videoPlaybackFailed) void loadBackgroundTexture(value)
})
watch(() => props.videoUrl, async value => {
  cancelVideoFrameTracking()
  videoPlaybackFailed = false
  videoUploadFailureCount = 0
  videoFrameDirty = false
  if (!value) {
    void loadBackgroundTexture(props.backgroundUrl)
    return
  }
  await nextTick()
  if ((videoSource.value?.readyState ?? 0) >= HTMLMediaElement.HAVE_CURRENT_DATA) {
    handleVideoLoaded()
  } else if (state) {
    // 视频解码期间保留已有图片或默认背景，避免出现纯色闪屏。
    void loadBackgroundTexture(props.backgroundUrl)
  }
})
watch(motionEnabled, () => startRenderer())
watch(liquidTone, () => startRenderer())
watch(liquidTransparency, () => startRenderer())

onMounted(() => {
  if (allowFullScreenWebGl) {
    canvas.value?.addEventListener('webglcontextlost', handleContextLost)
    canvas.value?.addEventListener('webglcontextrestored', handleContextRestored)
    initializeRenderer()
  } else {
    enterCssSafeMode('css-safe-windows-transparent')
  }
  ;(window as GlassQaWindow).__TOKEN_MANAGER_GLASS_QA__ = runRefractionQa
  ;(window as GlassQaWindow).__TOKEN_MANAGER_GLASS_SURFACE_AUDIT__ = runSurfaceAudit
  resizeObserver = new ResizeObserver(markRectsDirty)
  resizeObserver.observe(document.documentElement)
  mutationObserver = new MutationObserver(markRectsDirty)
  // 只观察结构变化；鼠标微光会频繁修改 class/style，但不会改变卡片几何位置。
  mutationObserver.observe(document.body, { childList: true, subtree: true })
  window.addEventListener('resize', markRectsDirty, { passive: true })
  window.addEventListener('scroll', markRectsDirty, { passive: true, capture: true })
  window.addEventListener('pointermove', updatePointer, { passive: true })
  window.addEventListener('token-manager-glass-render', renderOnce)
})

onUnmounted(() => {
  stopRenderer()
  lastTrackedMaterialSurfaces.forEach(surface => delete surface.dataset.liquidOpticalSurface)
  lastSuppressedNestedSurfaces.forEach(surface => delete surface.dataset.liquidNestedSuppressed)
  cancelVideoFrameTracking()
  canvas.value?.removeEventListener('webglcontextlost', handleContextLost)
  canvas.value?.removeEventListener('webglcontextrestored', handleContextRestored)
  resizeObserver?.disconnect()
  mutationObserver?.disconnect()
  window.removeEventListener('resize', markRectsDirty)
  window.removeEventListener('scroll', markRectsDirty, true)
  window.removeEventListener('pointermove', updatePointer)
  window.removeEventListener('token-manager-glass-render', renderOnce)
  if ((window as GlassQaWindow).__TOKEN_MANAGER_GLASS_QA__ === runRefractionQa) {
    delete (window as GlassQaWindow).__TOKEN_MANAGER_GLASS_QA__
  }
  if ((window as GlassQaWindow).__TOKEN_MANAGER_GLASS_SURFACE_AUDIT__ === runSurfaceAudit) {
    delete (window as GlassQaWindow).__TOKEN_MANAGER_GLASS_SURFACE_AUDIT__
  }
  releaseRendererState()
  document.documentElement.dataset.glassBackground = 'default'
  delete document.documentElement.dataset.glassMedia
  delete document.documentElement.dataset.glassRenderer
  document.documentElement.classList.remove('glass-webgl-active', 'glass-webgl-pending', 'glass-webgl-fallback')
})
</script>

<template>
  <video
    v-if="videoUrl"
    :key="videoUrl"
    ref="videoSource"
    class="liquid-video-source"
    crossorigin="anonymous"
    :src="videoUrl"
    autoplay
    muted
    loop
    playsinline
    preload="auto"
    aria-hidden="true"
    @loadeddata="handleVideoLoaded"
    @error="handleVideoPlaybackError"
  ></video>
  <canvas
    v-if="allowFullScreenWebGl"
    ref="canvas"
    class="liquid-glass-renderer"
    :class="{ compact }"
    aria-hidden="true"
  ></canvas>
</template>

<style scoped>
.liquid-video-source,
.liquid-glass-renderer {
  position: absolute;
  z-index: 0;
  inset: 0;
  width: 100%;
  height: 100%;
  display: block;
  pointer-events: none;
  contain: paint;
}

.liquid-video-source {
  object-fit: cover;
  background: #070a12;
}

.liquid-glass-renderer {
  opacity: 0;
  visibility: hidden;
  transition: opacity .18s cubic-bezier(.16, 1, .3, 1);
}

:global(.glass-webgl-active .liquid-glass-renderer) {
  opacity: 1;
  visibility: visible;
}

:global(.glass-webgl-active .liquid-video-source) {
  opacity: 0;
}

:global(.glass-webgl-pending .liquid-glass-renderer),
:global(.glass-webgl-fallback .liquid-glass-renderer) {
  opacity: 0 !important;
  visibility: hidden !important;
}

@media (prefers-reduced-motion: reduce) {
  .liquid-glass-renderer {
    transition: none;
  }
}
</style>
