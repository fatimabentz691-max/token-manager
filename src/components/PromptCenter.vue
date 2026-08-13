<script setup lang="ts">
import { computed, onUnmounted, ref, watch } from 'vue'
import ProviderMark from './ProviderMark.vue'

type PromptCategory = '编程'|'图像生成'|'写作'|'翻译'|'学习'|'研究'|'个人'
interface PromptRecord {
  id:string; title:string; description:string; body:string; category:PromptCategory; tags:string[];
  models:string[]; favorite:boolean; uses:number; lastUsed:string; createdAt:string;
}
const storageKey = 'token-manager-prompt-library-v1'
const categories:PromptCategory[] = ['编程','图像生成','写作','翻译','学习','研究','个人']
const modelOptions = ['Codex','Claude Code','DeepSeek','通义百炼','智谱 AI','Google Gemini']
const seed:PromptRecord[] = [
  { id:'seed-review',title:'严格代码审查',description:'发现正确性、性能与安全问题，并给出可执行修改。',body:'你是一名资深代码审查工程师。请先总结代码意图，再按严重级别列出问题，最后提供最小修改方案。不要改动无关逻辑。',category:'编程',tags:['代码审查','调试'],models:['Codex','Claude Code'],favorite:true,uses:12,lastUsed:new Date().toISOString(),createdAt:new Date().toISOString() },
  { id:'seed-refactor',title:'渐进式重构',description:'在保持行为不变的前提下拆分复杂模块。',body:'分析以下模块的职责边界与耦合点。给出三步渐进式重构计划，每一步都必须可独立测试和回滚。输出修改后的关键代码。',category:'编程',tags:['重构','架构'],models:['Codex','DeepSeek'],favorite:false,uses:8,lastUsed:new Date(Date.now()-86400000).toISOString(),createdAt:new Date().toISOString() },
]

function loadPrompts():PromptRecord[] {
  try { const value = JSON.parse(localStorage.getItem(storageKey) || 'null'); return Array.isArray(value) ? value : seed }
  catch { return seed }
}
const prompts = ref<PromptRecord[]>(loadPrompts())
const tab = ref<'全部'|'收藏'|'最近'|'常用'>('全部')
const category = ref<PromptCategory|'全部'>('全部')
const search = ref('')
const editing = ref<PromptRecord|null>(null)
const editorOpen = ref(false)
const copied = ref('')
const draft = ref<Omit<PromptRecord,'id'|'uses'|'lastUsed'|'createdAt'>>({
  title:'',description:'',body:'',category:'编程',tags:[],models:['Codex'],favorite:false,
})
const tagText = ref('')
let previousBodyOverflow = ''

watch(editorOpen, open => {
  if (open) {
    previousBodyOverflow = document.body.style.overflow
    document.body.style.overflow = 'hidden'
  } else {
    document.body.style.overflow = previousBodyOverflow
  }
})
onUnmounted(() => { document.body.style.overflow = previousBodyOverflow })

function persist(){ localStorage.setItem(storageKey,JSON.stringify(prompts.value)) }
const quality = computed(() => {
  const body = draft.value.body.trim()
  let score = 20
  const suggestions:string[] = []
  if (body.length >= 80) score += 20; else suggestions.push('补充任务背景与必要上下文')
  if (/你是|角色|专家|工程师/.test(body)) score += 14; else suggestions.push('明确模型需要扮演的角色')
  if (/输出|格式|表格|清单|步骤/.test(body)) score += 16; else suggestions.push('指定输出结构和格式')
  if (/不要|必须|限制|约束|仅/.test(body)) score += 15; else suggestions.push('加入边界条件和禁止事项')
  if (/示例|例如|输入|代码/.test(body)) score += 15; else suggestions.push('提供一个输入或期望输出示例')
  return { score:Math.min(100,score), suggestions }
})
const semanticTerms:Record<string,string[]>={
  '修复':['bug','调试','报错','错误'],'bug':['修复','调试','报错'],'重构':['架构','拆分','优化'],
  '写代码':['编程','代码','脚本','开发'],'翻译':['中英','语言','润色'],'总结':['摘要','归纳','提炼'],
  '图片':['图像','绘画','视觉'],'研究':['调研','分析','资料'],
}
const visible = computed(() => {
  let rows = prompts.value.filter(item => category.value === '全部' || item.category === category.value)
  if (tab.value === '收藏') rows = rows.filter(item => item.favorite)
  if (tab.value === '最近') rows = [...rows].sort((a,b) => +new Date(b.lastUsed)-+new Date(a.lastUsed))
  if (tab.value === '常用') rows = [...rows].sort((a,b) => b.uses-a.uses)
  const key = search.value.trim().toLowerCase()
  if (key) {
    const terms=[key,...Object.entries(semanticTerms).flatMap(([source,related])=>key.includes(source)?related:[])]
    rows = rows.filter(item => {
      const haystack=[item.title,item.description,item.body,item.category,...item.tags,...item.models].join(' ').toLowerCase()
      return terms.some(term=>haystack.includes(term.toLowerCase()))
    })
  }
  return rows
})
const totalUses = computed(() => prompts.value.reduce((sum,item)=>sum+item.uses,0))
const favoriteCount = computed(() => prompts.value.filter(item=>item.favorite).length)

function openEditor(item?:PromptRecord){
  editorOpen.value=true
  editing.value=item||null
  draft.value=item?{title:item.title,description:item.description,body:item.body,category:item.category,tags:[...item.tags],models:[...item.models],favorite:item.favorite}:{title:'',description:'',body:'',category:'编程',tags:[],models:['Codex'],favorite:false}
  tagText.value=draft.value.tags.join('、')
}
function savePrompt(){
  if(!draft.value.title.trim()||!draft.value.body.trim())return
  const tags=tagText.value.split(/[，,、\s]+/).filter(Boolean)
  if(editing.value){
    const index=prompts.value.findIndex(item=>item.id===editing.value?.id)
    prompts.value[index]={...prompts.value[index],...draft.value,tags}
  }else{
    const now=new Date().toISOString()
    prompts.value.unshift({...draft.value,tags,id:crypto.randomUUID(),uses:0,lastUsed:now,createdAt:now})
  }
  persist();editing.value=null;editorOpen.value=false
}
function removePrompt(id:string){ prompts.value=prompts.value.filter(item=>item.id!==id);persist();editing.value=null;editorOpen.value=false }
function toggleFavorite(item:PromptRecord){item.favorite=!item.favorite;persist()}
async function copyPrompt(item:PromptRecord){
  await navigator.clipboard.writeText(item.body)
  item.uses++;item.lastUsed=new Date().toISOString();copied.value=item.id;persist()
  window.setTimeout(()=>copied.value='',1200)
}
async function sharePrompt(item:PromptRecord){
  await navigator.clipboard.writeText(`${item.title}\n\n${item.body}`)
  copied.value=item.id;window.setTimeout(()=>copied.value='',1200)
}
function optimize(){
  const base=draft.value.body.trim()
  draft.value.body=`你是一名专注于${draft.value.category}任务的资深专家。\n\n任务目标：\n${base}\n\n输出要求：\n1. 先给出结论，再说明关键依据。\n2. 使用清晰的分段或步骤。\n3. 不补充未经提供的信息；不确定处明确标注。\n4. 最后给出可直接执行的下一步。`
}
</script>

<template>
  <section class="prompt-center">
    <header class="prompt-hero">
      <div><span>PROMPT LIBRARY</span><h2>Prompt 管理中心</h2><p class="tm-supplemental-description">本地保存、快速复用，并在发送前检查 Prompt 的清晰度与约束。</p></div>
      <button class="prompt-primary" type="button" @click="openEditor()">＋ 新建 Prompt</button>
    </header>
    <section class="prompt-stats">
      <div><small>本地模板</small><b>{{ prompts.length }}</b></div><div><small>累计使用</small><b>{{ totalUses }}</b></div><div><small>已收藏</small><b>{{ favoriteCount }}</b></div><div><small>数据位置</small><b>仅本机</b></div>
    </section>
    <div class="prompt-toolbar">
      <div class="prompt-tabs"><button v-for="item in ['全部','收藏','最近','常用'] as const" :key="item" :class="{active:tab===item}" @click="tab=item">{{item}}</button></div>
      <label><span aria-hidden="true">⌕</span><input v-model="search" placeholder="搜索名称、标签、内容或模型"></label>
    </div>
    <div class="prompt-layout">
      <aside class="prompt-categories"><span>分类</span><button :class="{active:category==='全部'}" @click="category='全部'"><b>全部</b><em>{{prompts.length}}</em></button><button v-for="item in categories" :key="item" :class="{active:category===item}" @click="category=item"><b>{{item}}</b><em>{{prompts.filter(row=>row.category===item).length}}</em></button></aside>
      <section>
        <div v-if="visible.length" class="prompt-grid">
          <article v-for="item in visible" :key="item.id" class="prompt-card">
            <header><span>{{item.category}}</span><button type="button" :aria-label="item.favorite?'取消收藏':'收藏'" @click="toggleFavorite(item)">{{item.favorite?'★':'☆'}}</button></header>
            <h3>{{item.title}}</h3><p>{{item.description||item.body}}</p>
            <div class="prompt-tags"><span v-for="tag in item.tags" :key="tag">#{{tag}}</span></div>
            <div class="prompt-models"><ProviderMark v-for="model in item.models.slice(0,3)" :key="model" :name="model==='Codex'?'OpenAI':model" /></div>
            <footer><small>{{item.uses}} 次使用</small><div><button type="button" @click="sharePrompt(item)">分享</button><button type="button" @click="openEditor(item)">编辑</button><button class="copy" type="button" @click="copyPrompt(item)">{{copied===item.id?'已复制':'复制'}}</button></div></footer>
          </article>
        </div>
        <div v-else class="prompt-empty"><b>没有找到 Prompt</b><p>换一个关键词或分类，也可以新建自己的第一个模板。</p><button @click="openEditor()">新建 Prompt</button></div>
      </section>
    </div>
    <Teleport to="body">
      <Transition name="prompt-modal">
        <div v-if="editorOpen" class="prompt-modal-backdrop" @mousedown.self="editorOpen=false">
          <section class="prompt-editor" role="dialog" aria-modal="true" aria-label="Prompt 编辑器">
          <header><div><span>LOCAL PROMPT EDITOR</span><h2>{{editing?'编辑 Prompt':'新建 Prompt'}}</h2></div><button aria-label="关闭" @click="editorOpen=false">×</button></header>
          <div class="editor-grid">
            <div class="editor-fields">
              <label>标题<input v-model="draft.title" placeholder="例如：严格代码审查"></label>
              <label>简短说明<input v-model="draft.description" placeholder="说明它最适合解决什么问题"></label>
              <label>Prompt 正文<textarea v-model="draft.body" rows="11" placeholder="写下角色、任务目标、上下文、输出格式与限制…"></textarea></label>
              <div class="editor-row"><label>分类<select v-model="draft.category"><option v-for="item in categories" :key="item">{{item}}</option></select></label><label>标签<input v-model="tagText" placeholder="重构、审查、前端"></label></div>
              <fieldset><legend>推荐模型</legend><label v-for="model in modelOptions" :key="model"><input v-model="draft.models" type="checkbox" :value="model">{{model}}</label></fieldset>
            </div>
            <aside class="quality-panel">
              <span>质量检查</span><div class="quality-score" :style="{'--score':quality.score*3.6+'deg'}"><b>{{quality.score}}</b><small>/100</small></div>
              <h3>{{quality.score>=85?'结构完整':quality.score>=65?'可以使用':'建议补充'}}</h3>
              <ul><li v-for="item in quality.suggestions" :key="item">{{item}}</li><li v-if="!quality.suggestions.length">角色、上下文、输出结构和限制都已明确</li></ul>
              <button type="button" @click="optimize">生成优化版本</button>
            </aside>
          </div>
          <footer><button v-if="editing" class="danger" @click="removePrompt(editing.id)">删除</button><span></span><button @click="editorOpen=false">取消</button><button class="prompt-primary" :disabled="!draft.title.trim()||!draft.body.trim()" @click="savePrompt">保存到本机</button></footer>
          </section>
        </div>
      </Transition>
    </Teleport>
  </section>
</template>

<style scoped>
.prompt-center{color:var(--tm-ink)}.prompt-hero{display:flex;align-items:flex-end;justify-content:space-between;gap:20px;padding:10px 2px 24px;animation:prompt-enter .3s cubic-bezier(.2,.8,.2,1) both}.prompt-hero span,.prompt-editor header span{color:var(--tm-accent);font-size:8px;font-weight:700;letter-spacing:.18em}.prompt-hero h2{margin:8px 0 0;font-size:26px;letter-spacing:-.04em}.prompt-hero p{margin:7px 0 0;color:var(--tm-muted);font-size:10px}.prompt-primary{border-color:var(--tm-accent)!important;background:var(--tm-accent)!important;color:var(--tm-bg)!important}.prompt-stats{display:grid;grid-template-columns:repeat(4,1fr);gap:10px;margin-bottom:14px}.prompt-stats div{display:grid;gap:6px;padding:15px;border:1px solid var(--tm-line);border-radius:16px;background:var(--tm-glass);animation:prompt-enter .3s cubic-bezier(.2,.8,.2,1) both}.prompt-stats div:nth-child(2){animation-delay:.04s}.prompt-stats div:nth-child(3){animation-delay:.08s}.prompt-stats div:nth-child(4){animation-delay:.12s}.prompt-stats small{color:var(--tm-muted);font-size:8px}.prompt-stats b{font-size:18px}.prompt-toolbar{display:flex;align-items:center;justify-content:space-between;gap:14px;margin-bottom:14px}.prompt-tabs{display:flex;padding:3px;border-radius:12px;background:var(--tm-surface)}.prompt-tabs button{padding:8px 12px;border:0;border-radius:9px;background:transparent;color:var(--tm-muted);font:inherit;font-size:8px}.prompt-tabs button.active{background:var(--tm-bg);color:var(--tm-ink);box-shadow:0 2px 8px color-mix(in srgb,var(--tm-ink) 8%,transparent)}.prompt-toolbar>label{display:flex;align-items:center;gap:7px;width:min(320px,42vw);padding:0 11px;border:1px solid var(--tm-line);border-radius:12px;background:var(--tm-glass)}.prompt-toolbar input{width:100%;padding:10px 0;border:0;outline:0;background:transparent;color:var(--tm-ink);font:inherit;font-size:9px}.prompt-layout{display:grid;grid-template-columns:160px 1fr;gap:14px}.prompt-categories{display:flex;flex-direction:column;gap:4px;padding:12px;border:1px solid var(--tm-line);border-radius:18px;background:var(--tm-glass)}.prompt-categories>span{padding:6px 8px;color:var(--tm-muted);font-size:8px}.prompt-categories button{display:flex;justify-content:space-between;padding:9px;border:0;border-radius:10px;background:transparent;color:var(--tm-muted);font:inherit;text-align:left}.prompt-categories button.active{background:var(--tm-surface-strong);color:var(--tm-ink)}.prompt-categories b{font-size:9px}.prompt-categories em{font-size:8px;font-style:normal}.prompt-grid{display:grid;grid-template-columns:repeat(2,minmax(0,1fr));gap:10px}.prompt-card{display:flex;min-height:228px;flex-direction:column;padding:16px;border:1px solid var(--tm-line);border-radius:18px;background:var(--tm-glass);animation:prompt-enter .3s cubic-bezier(.2,.8,.2,1) both;transition:transform .25s cubic-bezier(.2,.8,.2,1),border-color .25s ease}.prompt-card:nth-child(2n){animation-delay:.05s}.prompt-card:nth-child(3n){animation-delay:.1s}.prompt-card:hover{transform:translateY(-3px) scale(1.002);border-color:color-mix(in srgb,var(--tm-accent) 40%,var(--tm-line))}.prompt-card header{display:flex;justify-content:space-between}.prompt-card header span,.prompt-tags span{padding:4px 7px;border-radius:99px;background:var(--tm-surface-strong);color:var(--tm-muted);font-size:7px}.prompt-card header button{border:0;background:transparent;color:var(--tm-accent);font-size:16px}.prompt-card h3{margin:16px 0 7px;font-size:14px}.prompt-card>p{display:-webkit-box;overflow:hidden;margin:0;color:var(--tm-muted);font-size:9px;line-height:1.65;-webkit-box-orient:vertical;-webkit-line-clamp:3}.prompt-tags{display:flex;flex-wrap:wrap;gap:5px;margin-top:12px}.prompt-models{display:flex;gap:5px;margin-top:auto;padding-top:14px}.prompt-models :deep(.provider-mark){width:27px;height:27px;border-radius:8px}.prompt-models :deep(img){width:18px;height:18px}.prompt-card footer{display:flex;align-items:center;justify-content:space-between;margin-top:12px;padding-top:11px;border-top:1px solid var(--tm-line)}.prompt-card footer small{color:var(--tm-muted);font-size:7px}.prompt-card footer div{display:flex;gap:4px}.prompt-card footer button,.quality-panel button,.prompt-editor>footer button,.prompt-empty button,.prompt-hero>button{padding:7px 9px;border:1px solid var(--tm-line);border-radius:9px;background:transparent;color:var(--tm-ink);font:inherit;font-size:8px}.prompt-card footer .copy{background:var(--tm-ink);color:var(--tm-on-ink)}.prompt-empty{display:grid;place-items:center;min-height:320px;border:1px dashed var(--tm-line);border-radius:18px;text-align:center}.prompt-empty p{color:var(--tm-muted);font-size:9px}.prompt-modal-backdrop{position:fixed;z-index:100;inset:0;display:grid;place-items:center;padding:16px;background:rgba(0,0,0,.38);backdrop-filter:blur(14px)}.prompt-editor{display:grid;grid-template-rows:auto minmax(0,1fr) auto;width:min(880px,calc(100vw - 32px));height:min(680px,calc(100vh - 32px));max-height:calc(100vh - 32px);overflow:hidden;border:1px solid var(--tm-line);border-radius:24px;background:var(--tm-bg);box-shadow:var(--tm-shadow)}.prompt-editor>header{display:flex;align-items:center;justify-content:space-between;padding:16px 20px;border-bottom:1px solid var(--tm-line);background:color-mix(in srgb,var(--tm-bg) 92%,transparent);backdrop-filter:blur(18px)}.prompt-editor h2{margin:5px 0 0;font-size:20px}.prompt-editor>header button{display:grid;place-items:center;width:32px;height:32px;border:0;border-radius:10px;background:var(--tm-surface);color:var(--tm-ink);font-size:18px}.editor-grid{display:grid;grid-template-columns:minmax(0,1fr) 230px;gap:18px;min-height:0;padding:18px 20px;overflow:auto;overscroll-behavior:contain}.editor-fields{display:grid;gap:12px}.editor-fields>label,.editor-row label{display:grid;gap:6px;color:var(--tm-muted);font-size:8px}.editor-fields input,.editor-fields textarea,.editor-fields select{box-sizing:border-box;width:100%;padding:10px 11px;border:1px solid var(--tm-line);border-radius:11px;outline:0;background:var(--tm-surface);color:var(--tm-ink);font:inherit;font-size:9px;resize:vertical}.editor-fields input:focus,.editor-fields textarea:focus,.editor-fields select:focus{border-color:var(--tm-accent);box-shadow:0 0 0 4px color-mix(in srgb,var(--tm-accent) 12%,transparent)}.editor-row{display:grid;grid-template-columns:1fr 1fr;gap:10px}.editor-fields fieldset{display:flex;flex-wrap:wrap;gap:10px;margin:0;padding:10px;border:1px solid var(--tm-line);border-radius:12px}.editor-fields legend{padding:0 5px;color:var(--tm-muted);font-size:8px}.editor-fields fieldset label{display:inline-flex;align-items:center;gap:5px;font-size:8px}.editor-fields fieldset input[type="checkbox"]{width:14px;height:14px;padding:0;accent-color:var(--tm-accent)}.quality-panel{align-self:start;padding:16px;border:1px solid var(--tm-line);border-radius:18px;background:var(--tm-surface)}.quality-panel>span{color:var(--tm-muted);font-size:8px}.quality-score{--score:0deg;width:112px;height:112px;display:grid;place-items:center;margin:18px auto;border-radius:50%;background:conic-gradient(var(--tm-accent) var(--score),var(--tm-surface-strong) 0);animation:quality-pop .32s cubic-bezier(.2,.8,.2,1) both}.quality-score:before{content:"";grid-area:1/1;width:84px;height:84px;border-radius:50%;background:var(--tm-bg)}.quality-score b,.quality-score small{z-index:1;grid-area:1/1}.quality-score b{font-size:25px;transform:translateY(-7px)}.quality-score small{color:var(--tm-muted);font-size:8px;transform:translateY(12px)}.quality-panel h3{font-size:12px}.quality-panel ul{display:grid;gap:8px;padding-left:16px;color:var(--tm-muted);font-size:8px;line-height:1.5}.quality-panel button{width:100%;margin-top:8px}.prompt-editor>footer{display:flex;gap:8px;padding:13px 20px;border-top:1px solid var(--tm-line);background:color-mix(in srgb,var(--tm-bg) 94%,transparent);backdrop-filter:blur(18px)}.prompt-editor>footer span{flex:1}.prompt-editor>footer .danger{color:#ff453a}.prompt-editor>footer button:disabled{opacity:.45}.prompt-modal-enter-active,.prompt-modal-leave-active{transition:opacity .2s ease}.prompt-modal-enter-active .prompt-editor,.prompt-modal-leave-active .prompt-editor{transition:transform .28s cubic-bezier(.16,1,.3,1)}.prompt-modal-enter-from,.prompt-modal-leave-to{opacity:0}.prompt-modal-enter-from .prompt-editor,.prompt-modal-leave-to .prompt-editor{transform:scale(.965) translateY(8px)}@keyframes prompt-enter{from{opacity:0;transform:translateY(7px);filter:blur(3px)}to{opacity:1;transform:none;filter:none}}@keyframes quality-pop{from{opacity:.3;transform:scale(.88) rotate(-8deg)}to{opacity:1;transform:none}}:global(.motion-off) .prompt-center *,:global(.motion-off) .prompt-modal-backdrop *{animation:none!important;transition:none!important}@media(prefers-reduced-motion:reduce){.prompt-center *,.prompt-modal-backdrop *{animation:none!important;transition:none!important}}@media(max-width:860px){.prompt-layout{grid-template-columns:1fr}.prompt-categories{display:grid;grid-template-columns:repeat(4,1fr)}.prompt-categories>span{display:none}.editor-grid{grid-template-columns:1fr}.quality-panel{display:grid;grid-template-columns:120px 1fr;gap:10px}.quality-score{grid-row:1/4}}@media(max-width:620px){.prompt-grid,.prompt-stats{grid-template-columns:1fr 1fr}.prompt-toolbar{align-items:stretch;flex-direction:column}.prompt-toolbar>label{width:auto}}
</style>
