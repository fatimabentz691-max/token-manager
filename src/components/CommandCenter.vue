<script setup lang="ts">
import { computed } from 'vue'
import AnimatedNumber from './AnimatedNumber.vue'
import ProviderMark from './ProviderMark.vue'
import ParticleField from './ParticleField.vue'
import tokenManagerLogo from '../assets/token-manager-logo.png?url'

interface Activity { id:string; provider:string; model:string; tokens:number; cost:number; at:string }
const props = defineProps<{
  todayTokens:number
  todayCalls:number
  todayCost:number
  activeModels:number
  accountCount:number
  proxyCount:number
  ccConnected:boolean
  codexRemaining:number
  weekCost:number
  monthlyBudget:number
  activities:Activity[]
}>()
const emit = defineEmits<{ openDashboard:[]; openPrompts:[]; openArena:[]; startProxy:[] }>()

// 只有检测到真实账户、代理、CC Switch 或调用记录后才生成健康度，避免空数据时展示伪造分数。
const healthAvailable = computed(() => Boolean(
  props.proxyCount || props.ccConnected || props.accountCount || props.todayCalls || props.activities.length
))
const health = computed<number | null>(() => {
  if (!healthAvailable.value) return null
  let value = 100
  if (!props.proxyCount) value -= 12
  if (!props.ccConnected) value -= 4
  if (props.codexRemaining < 20) value -= 18
  if (props.monthlyBudget && props.weekCost * 4 > props.monthlyBudget) value -= 14
  return Math.max(42, value)
})
const healthLabel = computed(() => health.value === null ? '等待有效数据' : health.value >= 90 ? '系统状态优秀' : health.value >= 75 ? '系统状态良好' : '建议检查')
const healthDashOffset = computed(() => 314.159 * (1 - (health.value ?? 0) / 100))
const budgetPercent = computed(() => props.monthlyBudget ? Math.min(100, props.weekCost * 4 / props.monthlyBudget * 100) : 0)
const mostUsed = computed(() => props.activities[0]?.provider || '等待数据')
const recommendation = computed(() => {
  if (props.codexRemaining <= 20) return 'Codex 额度偏低，大段代码生成建议暂时切换至 DeepSeek 或通义千问。'
  if (!props.proxyCount) return '开启本地 API 代理后，可实时记录 Token、请求、缓存命中和消费。'
  return '当前数据链路稳定。复杂调试使用 Codex，批量生成优先选择高性价比模型。'
})
const hour = new Date().getHours()
const greeting = hour < 6 ? '夜深了' : hour < 12 ? '早上好' : hour < 18 ? '下午好' : '晚上好'
</script>

<template>
  <section class="command-center">
    <ParticleField />
    <div class="command-content">
      <header class="command-hero" data-stagger="1">
        <div>
          <span class="command-kicker">AI COMMAND CENTER</span>
          <h2>{{ greeting }}，开发者</h2>
          <p>所有 AI 模型、额度、Prompt 与状态，集中在一个本地控制中心。</p>
        </div>
        <div class="command-status" data-liquid-surface :class="{ live: proxyCount || ccConnected }">
          <i></i><span><small>LOCAL SYSTEM</small><b>{{ proxyCount || ccConnected ? '实时数据通道已就绪' : '等待连接数据通道' }}</b></span>
        </div>
      </header>

      <section class="command-grid">
        <article class="glass-card hero-metric" data-liquid-surface data-stagger="2">
          <div class="card-label"><span>今日 Token</span><em>实时</em></div>
          <strong><AnimatedNumber :value="todayTokens" /></strong>
          <p>{{ todayCalls.toLocaleString() }} 次调用 · ¥{{ todayCost.toFixed(2) }} 今日消费</p>
          <div class="metric-wave" aria-label="Token 活跃度">
            <i v-for="n in 18" :key="n" :style="{ height: `${18 + ((n * 19 + todayCalls) % 58)}%` }"></i>
          </div>
        </article>

        <article class="glass-card health-card" data-liquid-surface data-stagger="3">
          <div class="card-label"><span>AI 健康度</span><em>{{ healthLabel }}</em></div>
          <div class="health-visual">
            <div
              class="health-ring"
              :class="{ empty: health === null }"
              role="meter"
              aria-label="AI 数据链路健康度"
              aria-valuemin="0"
              aria-valuemax="100"
              :aria-valuenow="health ?? undefined"
              :aria-valuetext="health === null ? '等待有效数据' : `${health} 分`"
            >
              <svg viewBox="0 0 120 120" aria-hidden="true">
                <circle class="health-track" cx="60" cy="60" r="50" />
                <circle class="health-progress" cx="60" cy="60" r="50" :style="{ strokeDashoffset: healthDashOffset }" />
              </svg>
              <span v-if="health !== null"><b>{{ health }}</b><small>健康分</small></span>
              <span v-else class="health-waiting"><b>—</b><small>待检测</small></span>
            </div>
            <dl>
              <div><dt>代理通道</dt><dd>{{ proxyCount ? `${proxyCount} 条` : '未启用' }}</dd></div>
              <div><dt>CC Switch</dt><dd>{{ ccConnected ? '已接管' : '待机' }}</dd></div>
              <div><dt>账户</dt><dd>{{ accountCount }}</dd></div>
            </dl>
          </div>
        </article>

        <article class="glass-card compact-card" data-liquid-surface data-stagger="4">
          <div class="card-label"><span>最常用模型</span><em>今日</em></div>
          <div class="model-focus"><ProviderMark :name="mostUsed === 'Codex' ? 'OpenAI' : mostUsed" /><span><b>{{ mostUsed }}</b><small>{{ activeModels }} 个模型活跃</small></span></div>
          <button type="button" @click="emit('openDashboard')">查看模型用量</button>
        </article>

        <article class="glass-card compact-card" data-liquid-surface data-stagger="5">
          <div class="card-label"><span>Codex 周额度</span><em>本地观测</em></div>
          <div class="quota-row"><strong>{{ codexRemaining }}%</strong><div><i :style="{ width: `${codexRemaining}%` }"></i></div></div>
          <small class="card-note">以客户端 rate_limits 为准；缺失时使用个人预算估算。</small>
        </article>

        <article class="glass-card activity-card" data-liquid-surface data-stagger="6">
          <div class="card-label"><span>最近 AI 活动</span><button type="button" @click="emit('openDashboard')">全部记录</button></div>
          <div v-if="activities.length" class="activity-list">
            <div v-for="item in activities.slice(0,5)" :key="item.id">
              <ProviderMark :name="item.provider === 'Codex' ? 'OpenAI' : item.provider" />
              <span><b>{{ item.model }}</b><small>{{ new Date(item.at).toLocaleString('zh-CN', { month:'2-digit', day:'2-digit', hour:'2-digit', minute:'2-digit' }) }}</small></span>
              <em>{{ item.tokens.toLocaleString() }} Token</em>
            </div>
          </div>
          <div v-else class="command-empty"><img :src="tokenManagerLogo" alt=""><span><b>还没有活动记录</b><small>使用 Codex，或开启 API 代理后会自动记录。</small></span></div>
        </article>

        <article class="glass-card recommendation-card" data-liquid-surface data-stagger="7">
          <div class="card-label"><span>智能建议</span><em>本地分析</em></div>
          <p>{{ recommendation }}</p>
          <div class="budget-line"><span><small>月度预算预测</small><b>{{ budgetPercent.toFixed(0) }}%</b></span><i><em :style="{ width: `${budgetPercent}%` }"></em></i></div>
          <div class="command-actions">
            <button type="button" @click="emit('openPrompts')">Prompt 中心</button>
            <button type="button" @click="emit('openArena')">Arena 排行榜</button>
            <button v-if="!proxyCount" class="primary" type="button" @click="emit('startProxy')">开启实时代理</button>
          </div>
        </article>
      </section>
    </div>
  </section>
</template>

<style scoped>
.command-center{position:relative;width:100%;max-width:100%;min-height:calc(100vh - 172px);overflow:hidden;border:1px solid var(--tm-line);border-radius:28px;background:var(--tm-bg);color:var(--tm-ink);isolation:isolate;container-type:inline-size;transition:background-color .28s ease,color .28s ease,border-color .28s ease}.command-center:before{content:"";position:absolute;inset:0;background:linear-gradient(135deg,color-mix(in srgb,var(--tm-glow) 16%,transparent),transparent 42%,color-mix(in srgb,var(--tm-accent) 10%,transparent));pointer-events:none}.command-content{position:relative;z-index:1;width:100%;max-width:100%;padding:28px}.command-hero{display:flex;align-items:flex-end;justify-content:space-between;gap:24px;margin-bottom:24px;animation:hero-arrive .3s cubic-bezier(.2,.8,.2,1) both}.command-kicker{display:block;margin-bottom:8px;color:var(--tm-accent);font-size:9px;font-weight:700;letter-spacing:.2em}.command-hero h2{margin:0;font-size:29px;line-height:1.12;letter-spacing:-.04em}.command-hero p{margin:9px 0 0;color:var(--tm-muted);font-size:11px}.command-status{display:flex;align-items:center;gap:10px;padding:11px 13px;border:1px solid var(--tm-line);border-radius:15px;background:var(--tm-glass);backdrop-filter:blur(20px);animation:status-arrive .32s .06s cubic-bezier(.2,.8,.2,1) both}.command-status>i{width:8px;height:8px;border-radius:50%;background:var(--tm-muted);box-shadow:0 0 0 5px color-mix(in srgb,var(--tm-muted) 12%,transparent)}.command-status.live>i{background:var(--tm-accent);box-shadow:0 0 0 5px color-mix(in srgb,var(--tm-accent) 16%,transparent),0 0 18px color-mix(in srgb,var(--tm-accent) 55%,transparent);animation:status-breathe 3.2s ease-in-out infinite}.command-status span{display:grid;gap:2px}.command-status small{color:var(--tm-muted);font-size:7px;letter-spacing:.11em}.command-status b{font-size:9px}.command-grid{display:grid;width:100%;max-width:100%;grid-template-columns:minmax(0,1.15fr) minmax(0,.85fr) minmax(0,.72fr);gap:14px}.glass-card{min-width:0;padding:18px;border:1px solid var(--tm-line);border-radius:20px;background:var(--tm-glass);box-shadow:inset 0 1px 0 color-mix(in srgb,var(--tm-ink) 8%,transparent);backdrop-filter:blur(24px);transition:transform .28s cubic-bezier(.2,.8,.2,1),border-color .28s ease,background .28s ease,box-shadow .28s ease}.glass-card:hover{transform:translateY(-3px) scale(1.003);border-color:color-mix(in srgb,var(--tm-accent) 45%,var(--tm-line));background:var(--tm-glass-strong);box-shadow:0 14px 34px color-mix(in srgb,var(--tm-glow) 12%,transparent)}.card-label{display:flex;align-items:center;justify-content:space-between;gap:10px}.card-label>span{font-size:10px;font-weight:700}.card-label em,.card-label button{border:0;background:transparent;color:var(--tm-muted);font:inherit;font-size:8px;font-style:normal}.hero-metric strong{display:block;margin-top:18px;font-size:38px;line-height:1;letter-spacing:-.06em}.hero-metric p{margin:7px 0 0;color:var(--tm-muted);font-size:9px}.metric-wave{height:64px;display:flex;align-items:flex-end;gap:5px;margin-top:20px;padding:10px 2px 0;border-top:1px solid var(--tm-line)}.metric-wave i{flex:1;min-width:3px;border-radius:99px;background:var(--tm-accent);opacity:.65;transform-origin:bottom;animation:command-bar .35s cubic-bezier(.2,.8,.2,1) both}.metric-wave i:nth-child(3n+1){animation-delay:.04s}.metric-wave i:nth-child(3n+2){animation-delay:.08s}.metric-wave i:nth-child(3n){animation-delay:.12s}.health-visual{display:grid;grid-template-columns:116px 1fr;align-items:center;gap:14px;margin-top:20px}.health-ring{--value:0deg;position:relative;width:108px;height:108px;display:grid;place-items:center;border-radius:50%;background:conic-gradient(var(--tm-accent) var(--value),var(--tm-surface-strong) 0);box-shadow:0 0 34px color-mix(in srgb,var(--tm-accent) 16%,transparent);animation:ring-breathe 3.6s ease-in-out infinite}.health-ring:before{content:"";grid-area:1/1;width:82px;height:82px;border-radius:50%;background:var(--tm-bg);box-shadow:inset 0 0 0 1px var(--tm-line)}.health-ring span{z-index:1;grid-area:1/1;display:grid;text-align:center}.health-ring b{font-size:27px}.health-ring small{color:var(--tm-muted);font-size:8px}.health-card dl{display:grid;gap:10px;margin:0}.health-card dl div{display:flex;justify-content:space-between;gap:8px;padding-bottom:9px;border-bottom:1px solid var(--tm-line);animation:list-arrive .26s cubic-bezier(.2,.8,.2,1) both}.health-card dl div:nth-child(2){animation-delay:.05s}.health-card dl div:nth-child(3){animation-delay:.1s}.health-card dt{color:var(--tm-muted);font-size:8px}.health-card dd{margin:0;font-size:9px;font-weight:700}.compact-card{min-height:145px}.model-focus{display:flex;align-items:center;gap:11px;margin:20px 0}.model-focus span{display:grid;gap:3px}.model-focus b{font-size:13px}.model-focus small,.card-note{color:var(--tm-muted);font-size:8px;line-height:1.5}.compact-card>button,.command-actions button{padding:8px 10px;border:1px solid var(--tm-line);border-radius:10px;background:transparent;color:var(--tm-ink);font:inherit;font-size:8px}.quota-row{margin-top:21px}.quota-row strong{font-size:27px}.quota-row>div{height:7px;margin:13px 0 10px;overflow:hidden;border-radius:99px;background:var(--tm-surface-strong)}.quota-row i{position:relative;display:block;height:100%;border-radius:inherit;background:var(--tm-accent);transform-origin:left;animation:liquid-fill .35s cubic-bezier(.2,.8,.2,1) both;transition:width .35s cubic-bezier(.2,.8,.2,1)}.quota-row i:after{content:"";position:absolute;top:0;right:0;width:8px;height:100%;border-radius:99px;background:color-mix(in srgb,var(--tm-bg) 55%,transparent);animation:liquid-glint .35s .18s ease-out both}.activity-card{grid-column:span 2}.activity-list{display:grid;margin-top:13px}.activity-list>div{display:grid;grid-template-columns:32px 1fr auto;align-items:center;gap:10px;padding:10px 0;border-top:1px solid var(--tm-line);animation:list-arrive .28s cubic-bezier(.2,.8,.2,1) both}.activity-list>div:nth-child(2){animation-delay:.04s}.activity-list>div:nth-child(3){animation-delay:.08s}.activity-list>div:nth-child(4){animation-delay:.12s}.activity-list>div:nth-child(5){animation-delay:.16s}.activity-list :deep(.provider-mark){width:30px;height:30px}.activity-list span{display:grid;gap:2px}.activity-list b{font-size:9px}.activity-list small{color:var(--tm-muted);font-size:7px}.activity-list em{font-size:8px;font-style:normal;font-weight:700}.recommendation-card p{min-height:62px;margin:20px 0;color:var(--tm-muted);font-size:10px;line-height:1.7}.budget-line span{display:flex;justify-content:space-between}.budget-line small{color:var(--tm-muted);font-size:8px}.budget-line b{font-size:9px}.budget-line>i{display:block;height:6px;margin-top:8px;overflow:hidden;border-radius:99px;background:var(--tm-surface-strong)}.budget-line>i em{display:block;height:100%;border-radius:inherit;background:var(--tm-accent);transform-origin:left;animation:liquid-fill .35s cubic-bezier(.2,.8,.2,1) both}.command-actions{display:flex;flex-wrap:wrap;gap:7px;margin-top:18px}.command-actions .primary{border-color:var(--tm-accent);background:var(--tm-accent);color:var(--tm-bg)}.command-empty{display:flex;align-items:center;gap:12px;margin-top:16px;padding:18px;border-radius:14px;background:var(--tm-surface)}.command-empty img{width:34px;height:34px;border-radius:10px}.command-empty span{display:grid;gap:3px}.command-empty b{font-size:9px}.command-empty small{color:var(--tm-muted);font-size:8px}.command-grid [data-stagger]{animation:command-enter .34s cubic-bezier(.2,.8,.2,1) both;animation-delay:calc(var(--index,0)*35ms)}[data-stagger="2"]{--index:2}[data-stagger="3"]{--index:3}[data-stagger="4"]{--index:4}[data-stagger="5"]{--index:5}[data-stagger="6"]{--index:6}[data-stagger="7"]{--index:7}@keyframes hero-arrive{from{opacity:0;transform:translateY(7px);filter:blur(3px)}to{opacity:1;transform:none;filter:none}}@keyframes status-arrive{from{opacity:0;transform:scale(.96) translateY(3px)}to{opacity:1;transform:none}}@keyframes command-enter{from{opacity:0;transform:translateY(8px) scale(.992);filter:blur(4px)}to{opacity:1;transform:none;filter:none}}@keyframes command-bar{from{transform:scaleY(.05);opacity:0}to{transform:scaleY(1);opacity:.65}}@keyframes liquid-fill{from{transform:scaleX(.04);opacity:.4}to{transform:scaleX(1);opacity:1}}@keyframes liquid-glint{from{opacity:0;transform:translateX(-10px)}to{opacity:.75;transform:none}}@keyframes list-arrive{from{opacity:0;transform:translateX(-5px)}to{opacity:1;transform:none}}@keyframes ring-breathe{0%,100%{box-shadow:0 0 22px color-mix(in srgb,var(--tm-accent) 10%,transparent)}50%{box-shadow:0 0 38px color-mix(in srgb,var(--tm-accent) 20%,transparent)}}@keyframes status-breathe{50%{box-shadow:0 0 0 6px color-mix(in srgb,var(--tm-accent) 10%,transparent),0 0 22px color-mix(in srgb,var(--tm-accent) 45%,transparent)}}@container(max-width:980px){.command-grid{grid-template-columns:1fr 1fr}.activity-card{grid-column:auto}.hero-metric{grid-column:span 2}}@container(max-width:650px){.command-content{padding:18px}.command-hero{align-items:flex-start;flex-direction:column}.command-grid{grid-template-columns:1fr}.hero-metric{grid-column:auto}.health-visual{grid-template-columns:110px 1fr}}:global(.motion-off) .command-center *{animation:none!important;transition:none!important}@media(prefers-reduced-motion:reduce){.command-center *{animation:none!important;transition:none!important}}
.health-ring{position:relative;width:108px;height:108px;display:grid;place-items:center;border-radius:50%;background:transparent;box-shadow:none;animation:none}.health-ring::before{display:none}.health-ring svg{grid-area:1/1;width:100%;height:100%;overflow:visible;transform:rotate(-90deg)}.health-ring circle{fill:none;stroke-width:8}.health-track{stroke:color-mix(in srgb,var(--tm-muted) 16%,transparent)}.health-progress{stroke:var(--tm-accent);stroke-linecap:round;stroke-dasharray:314.159;stroke-dashoffset:314.159;filter:drop-shadow(0 0 8px color-mix(in srgb,var(--tm-accent) 28%,transparent));transition:stroke-dashoffset .35s cubic-bezier(.2,.8,.2,1)}.health-ring.empty .health-progress{opacity:0}.health-ring span{z-index:1;grid-area:1/1;display:grid;gap:1px;text-align:center;line-height:1}.health-ring b{font-size:25px;letter-spacing:-.04em}.health-ring small{margin-top:4px;color:var(--tm-muted);font-size:7px;font-weight:650;letter-spacing:.04em}.health-ring .health-waiting b{color:var(--tm-muted);font-size:24px}.health-card .card-label em{padding:5px 8px;border-radius:999px;background:color-mix(in srgb,var(--tm-accent) 8%,transparent);color:var(--tm-muted)}
</style>
