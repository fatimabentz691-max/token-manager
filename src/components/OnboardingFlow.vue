<script setup lang="ts">
import { Check, ChevronRight, FileSearch, KeyRound, ShieldCheck, X } from '@lucide/vue'

defineProps<{ open: boolean; detected: { codex: boolean; claude: boolean; opencode: boolean } }>()
const emit = defineEmits<{ finish: []; skip: []; openAccounts: [] }>()
</script>

<template>
  <Transition name="onboarding-fade">
    <div v-if="open" class="onboarding-backdrop">
      <section class="onboarding-card" role="dialog" aria-modal="true" aria-label="首次接入向导">
        <button class="skip-icon" aria-label="跳过向导" @click="emit('skip')"><X :size="17" /></button>
        <span class="eyebrow">WELCOME TO TOKEN MANAGER</span>
        <h1>先连接你的 AI 数据</h1>
        <p class="lead">Token Manager 只统计真实接入的数据。完成下面任意一步后，仪表盘才会开始显示用量。</p>
        <div class="detected-grid">
          <article v-for="item in [
            { key: 'codex', title: 'Codex', note: '本地日志与客户端额度' },
            { key: 'claude', title: 'Claude Code', note: '本地会话用量' },
            { key: 'opencode', title: 'OpenCode', note: 'SQLite / JSON 本地记录' },
          ]" :key="item.key" :class="{ found: detected[item.key as keyof typeof detected] }">
            <FileSearch :size="18" />
            <div><b>{{ item.title }}</b><span>{{ item.note }}</span></div>
            <Check v-if="detected[item.key as keyof typeof detected]" :size="17" />
            <small v-else>未发现</small>
          </article>
        </div>
        <div class="privacy-row"><ShieldCheck :size="18" /><span>日志、代码和密钥不会上传；API Key 使用 Windows DPAPI 加密。</span></div>
        <div class="actions">
          <button class="secondary" @click="emit('skip')">稍后设置</button>
          <button @click="emit('openAccounts')"><KeyRound :size="16" />添加账户或本地数据源<ChevronRight :size="16" /></button>
          <button class="primary" @click="emit('finish')">使用已检测的数据进入高级模式</button>
        </div>
      </section>
    </div>
  </Transition>
</template>

<style scoped>
.onboarding-backdrop{position:fixed;z-index:400;inset:0;display:grid;padding:24px;place-items:center;background:rgba(18,18,20,.2);backdrop-filter:blur(12px)}.onboarding-card{position:relative;width:min(680px,100%);padding:38px;border:1px solid color-mix(in srgb,var(--tm-line) 80%,transparent);border-radius:24px;background:color-mix(in srgb,var(--tm-bg) 96%,transparent);box-shadow:0 30px 90px rgba(0,0,0,.15)}.skip-icon{position:absolute;top:20px;right:20px;display:grid;width:36px;height:36px;padding:0;place-items:center;border-radius:12px}.eyebrow{font-size:10px;font-weight:750;letter-spacing:.18em}h1{margin:10px 0 8px;font-size:38px;letter-spacing:-.05em}.lead{max-width:560px;margin:0 0 24px;color:var(--tm-muted);line-height:1.65}.detected-grid{display:grid;gap:10px}.detected-grid article{display:flex;align-items:center;gap:12px;padding:14px;border:1px solid var(--tm-line);border-radius:14px;background:var(--tm-card)}.detected-grid article>div{display:grid;flex:1}.detected-grid span,.detected-grid small{color:var(--tm-muted);font-size:11px}.detected-grid .found{border-color:color-mix(in srgb,#34c759 35%,var(--tm-line))}.privacy-row{display:flex;gap:10px;margin:18px 0;padding:13px 15px;border-radius:14px;background:var(--tm-surface);color:var(--tm-muted);font-size:12px;line-height:1.5}.actions{display:flex;flex-wrap:wrap;gap:10px}.actions button{display:flex;align-items:center;justify-content:center;gap:7px}.actions .primary{flex:1 1 240px}.onboarding-fade-enter-active,.onboarding-fade-leave-active{transition:opacity .22s ease}.onboarding-fade-enter-from,.onboarding-fade-leave-to{opacity:0}@media(max-width:620px){.onboarding-card{padding:28px 22px}h1{font-size:30px}.actions button{width:100%}}
</style>
