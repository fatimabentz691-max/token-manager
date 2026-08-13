<script setup lang="ts">
import { Check, ChevronLeft, ChevronRight, FileSearch, Gauge, KeyRound, MonitorUp, ShieldCheck, X } from '@lucide/vue'
import { ref, watch } from 'vue'

const props = defineProps<{ open: boolean; detected: { codex: boolean; claude: boolean; opencode: boolean } }>()
const emit = defineEmits<{ finish: []; skip: []; openAccounts: []; selectMode: [mode: 'simple' | 'advanced']; detect: [] }>()
const step = ref(1)
const path = ref<'local' | 'api'>('local')
const mode = ref<'simple' | 'advanced'>('advanced')
watch(() => props.open, value => { if (value) step.value = 1 })
function next() { if (step.value < 5) step.value += 1; else { emit('selectMode', mode.value); emit('finish') } }
</script>

<template>
  <Transition name="onboarding-fade">
    <div v-if="open" class="onboarding-backdrop">
      <section class="onboarding-card" role="dialog" aria-modal="true" aria-label="首次接入向导">
        <button class="skip-icon" aria-label="跳过向导" @click="emit('skip')"><X :size="17" /></button>
        <div class="step-track" aria-label="向导进度"><i v-for="index in 5" :key="index" :class="{ active: index <= step }"></i></div>
        <span class="eyebrow">TOKEN MANAGER · {{ step }}/5</span>

        <template v-if="step === 1">
          <ShieldCheck class="hero-icon" :size="30" /><h1>所有监控从本机开始</h1>
          <p class="lead">Token Manager 只统计真实接入的数据。日志、代码与密钥不会上传，API Key 使用当前 Windows 用户的 DPAPI 加密。</p>
          <div class="privacy-grid"><span><b>纯本地统计</b><small>不读取提示词、回复正文或代码</small></span><span><b>真实来源标识</b><small>官方、代理、日志与估算不会混淆</small></span></div>
        </template>

        <template v-else-if="step === 2">
          <FileSearch class="hero-icon" :size="30" /><h1>检测本机开发 Agent</h1>
          <p class="lead">路径会使用 %USERPROFILE%、%APPDATA% 等模板，在每台电脑上按当前 Windows 用户重新定位。</p>
          <div class="detected-grid">
            <article v-for="item in [{key:'codex',title:'Codex',note:'本地日志与客户端额度'},{key:'claude',title:'Claude Code',note:'本地会话用量'},{key:'opencode',title:'OpenCode CLI',note:'SQLite / JSON 本地记录'}]" :key="item.key" :class="{found:detected[item.key as keyof typeof detected]}"><FileSearch :size="18" /><div><b>{{item.title}}</b><span>{{item.note}}</span></div><Check v-if="detected[item.key as keyof typeof detected]" :size="17" /><small v-else>未发现</small></article>
          </div>
          <button class="inline-action" @click="emit('detect')">重新执行真实检测</button>
        </template>

        <template v-else-if="step === 3">
          <KeyRound class="hero-icon" :size="30" /><h1>选择主要监控方式</h1>
          <p class="lead">两种方式以后都能在“账户与模型”继续添加。</p>
          <div class="choice-grid"><button :class="{active:path==='local'}" @click="path='local'"><b>仅本地监控</b><span>Codex、Claude Code、OpenCode CLI 等</span></button><button :class="{active:path==='api'}" @click="path='api'"><b>API 账户监控</b><span>加密保存密钥并启用实时代理</span></button></div>
        </template>

        <template v-else-if="step === 4">
          <MonitorUp class="hero-icon" :size="30" /><h1>{{path==='local'?'确认数据路径':'连接 API 账户'}}</h1>
          <p class="lead">接入会使用软件现有的真实配置页面，不会创建一套无法生效的演示数据。</p>
          <button class="connect-action" @click="emit('openAccounts')">{{path==='local'?'打开 Agent 路径与监控设置':'添加并测试 API 账户'}}<ChevronRight :size="16" /></button>
          <small class="safe-note">CC Switch 运行时默认监控共存，不写入 OPENAI_BASE_URL 等全局环境变量。</small>
        </template>

        <template v-else>
          <Gauge class="hero-icon" :size="30" /><h1>选择你的界面复杂度</h1>
          <p class="lead">简单模式隐藏补充说明与高级调试；高级模式保留完整分析能力。</p>
          <div class="choice-grid"><button :class="{active:mode==='simple'}" @click="mode='simple'"><b>简单模式</b><span>核心数据与轻量说明入口</span></button><button :class="{active:mode==='advanced'}" @click="mode='advanced'"><b>高级模式</b><span>完整图表、来源与代理诊断</span></button></div>
          <div class="entry-list"><span>仪表盘</span><span>监控健康</span><span>悬浮窗</span></div>
        </template>

        <footer><button class="secondary" @click="emit('skip')">稍后设置</button><button v-if="step>1" class="secondary" @click="step--"><ChevronLeft :size="16" />上一步</button><button class="primary" @click="next">{{step===5?'完成并进入':'继续'}}<ChevronRight :size="16" /></button></footer>
      </section>
    </div>
  </Transition>
</template>

<style scoped>
.onboarding-backdrop{position:fixed;z-index:400;inset:0;display:grid;padding:24px;place-items:center;background:rgba(18,18,20,.2);backdrop-filter:blur(12px)}.onboarding-card{position:relative;width:min(700px,100%);min-height:520px;padding:36px;border:1px solid color-mix(in srgb,var(--tm-line) 80%,transparent);border-radius:24px;background:color-mix(in srgb,var(--tm-bg) 96%,transparent);box-shadow:0 30px 90px rgba(0,0,0,.15);font-family:"PingFang SC","Microsoft YaHei",sans-serif}.skip-icon{position:absolute;top:20px;right:20px;display:grid;width:36px;height:36px;padding:0;place-items:center;border-radius:12px}.step-track{display:flex;width:190px;gap:6px;margin-bottom:24px}.step-track i{height:4px;flex:1;border-radius:99px;background:var(--tm-line)}.step-track i.active{background:var(--tm-ink)}.eyebrow{font-size:10px;font-weight:750;letter-spacing:.16em}.hero-icon{margin-top:26px}h1{margin:10px 0 8px;font-size:34px;letter-spacing:-.04em}.lead{max-width:590px;margin:0 0 22px;color:var(--tm-muted);line-height:1.7}.privacy-grid,.choice-grid{display:grid;grid-template-columns:1fr 1fr;gap:10px}.privacy-grid span,.choice-grid button{display:grid;gap:5px;padding:17px;border:1px solid var(--tm-line);border-radius:15px;background:var(--tm-card);color:var(--tm-ink);text-align:left}.privacy-grid small,.choice-grid span,.safe-note{color:var(--tm-muted);font-size:11px;line-height:1.55}.choice-grid button.active{border-color:var(--tm-ink);box-shadow:inset 0 0 0 1px var(--tm-ink)}.detected-grid{display:grid;gap:9px}.detected-grid article{display:flex;align-items:center;gap:12px;padding:13px;border:1px solid var(--tm-line);border-radius:14px;background:var(--tm-card)}.detected-grid article>div{display:grid;flex:1}.detected-grid span,.detected-grid small{color:var(--tm-muted);font-size:11px}.detected-grid .found{border-color:color-mix(in srgb,#34c759 35%,var(--tm-line))}.inline-action,.connect-action{display:flex;align-items:center;justify-content:center;gap:8px;margin-top:13px}.connect-action{width:100%;min-height:54px}.entry-list{display:flex;flex-wrap:wrap;gap:8px;margin-top:18px}.entry-list span{padding:7px 10px;border-radius:999px;background:var(--tm-surface);font-size:11px}footer{position:absolute;right:36px;bottom:32px;left:36px;display:flex;justify-content:flex-end;gap:9px}footer button{display:flex;align-items:center;justify-content:center;gap:6px}.onboarding-fade-enter-active,.onboarding-fade-leave-active{transition:opacity .22s ease}.onboarding-fade-enter-from,.onboarding-fade-leave-to{opacity:0}@media(max-width:620px){.onboarding-card{min-height:600px;padding:28px 22px}h1{font-size:28px}.privacy-grid,.choice-grid{grid-template-columns:1fr}footer{right:22px;bottom:22px;left:22px;flex-wrap:wrap}}@media(prefers-reduced-motion:reduce){.onboarding-fade-enter-active,.onboarding-fade-leave-active{transition:none}}
</style>
