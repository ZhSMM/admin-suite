<template>
  <div class="page-container">
    <div class="page-header">
      <h2>{{ t('tools.diff.title') }}</h2>
      <el-space>
        <el-radio-group v-model="view" size="small">
          <el-radio-button value="split">{{ t('tools.diff.split') }}</el-radio-button>
          <el-radio-button value="unified">{{ t('tools.diff.unified') }}</el-radio-button>
        </el-radio-group>
        <el-radio-group v-model="granularity" size="small">
          <el-radio-button value="line">{{ t('tools.diff.line') }}</el-radio-button>
          <el-radio-button value="word">{{ t('tools.diff.word') }}</el-radio-button>
          <el-radio-button value="char">{{ t('tools.diff.char') }}</el-radio-button>
        </el-radio-group>
        <el-button :icon="DocumentCopy" @click="copyDiff">{{ t('common.copy') }}</el-button>
        <el-button :icon="Delete" @click="clear">{{ t('common.delete') }}</el-button>
      </el-space>
    </div>

    <el-row :gutter="12">
      <el-col :span="12">
        <el-card shadow="never">
          <template #header>
            <div class="card-header">
              <strong>{{ t('tools.diff.labelA') }} — {{ t('tools.diff.original') }}</strong>
              <span class="hint">{{ left.length }} {{ t('tools.diff.chars') }}</span>
            </div>
          </template>
          <el-input v-model="left" type="textarea" :rows="12" spellcheck="false" />
        </el-card>
      </el-col>
      <el-col :span="12">
        <el-card shadow="never">
          <template #header>
            <div class="card-header">
              <strong>{{ t('tools.diff.labelB') }} — {{ t('tools.diff.modified') }}</strong>
              <span class="hint">{{ right.length }} {{ t('tools.diff.chars') }}</span>
            </div>
          </template>
          <el-input v-model="right" type="textarea" :rows="12" spellcheck="false" />
        </el-card>
      </el-col>
    </el-row>

    <el-row :gutter="12" style="margin-top: 16px">
      <el-col :span="24">
        <el-card shadow="never">
          <template #header>
            <div class="card-header">
              <strong>{{ t('tools.diff.result') }}</strong>
              <span class="hint">
                <el-tag type="success" size="small">+{{ stats.add }}</el-tag>
                <el-tag type="danger" size="small">-{{ stats.del }}</el-tag>
                {{ stats.same }} {{ t('tools.diff.unchanged') }}
              </span>
            </div>
          </template>

          <div v-if="view === 'split'" class="split">
            <div class="pane">
              <div
                v-for="(part, i) in leftParts"
                :key="'l' + i"
                class="line"
                :class="{ removed: part.removed, added: false }"
              >
                <span class="ln">{{ i + 1 }}</span>
                <span class="content" v-html="formatLine(part.left)"></span>
              </div>
            </div>
            <div class="pane">
              <div
                v-for="(part, i) in rightParts"
                :key="'r' + i"
                class="line"
                :class="{ added: part.added, removed: false }"
              >
                <span class="ln">{{ i + 1 }}</span>
                <span class="content" v-html="formatLine(part.right)"></span>
              </div>
            </div>
          </div>

          <pre v-else class="unified">{{ unifiedText }}</pre>
        </el-card>
      </el-col>
    </el-row>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { ElMessage } from 'element-plus'
import { diffLines, diffWordsWithSpace, diffChars } from 'diff'
import { DocumentCopy, Delete } from '@element-plus/icons-vue'

const { t } = useI18n()

const view = ref<'split' | 'unified'>('split')
const granularity = ref<'line' | 'word' | 'char'>('line')

const left = ref(`function greet(name) {
  console.log("Hello, " + name);
  return name;
}

greet("World");
`)
const right = ref(`function greet(name, greeting = "Hello") {
  console.log(\`\${greeting}, \${name}!\`);
  return name;
}

// greet the team
greet("World");
`)

const parts = computed(() => {
  switch (granularity.value) {
    case 'word': return diffWordsWithSpace(left.value, right.value)
    case 'char': return diffChars(left.value, right.value)
    default: return diffLines(left.value, right.value)
  }
})

const stats = computed(() => {
  let add = 0, del = 0, same = 0
  for (const p of parts.value) {
    if (p.added) add += p.value.length
    else if (p.removed) del += p.value.length
    else same += p.value.length
  }
  return { add, del, same }
})

// Split view: project parts into two columns.
const leftParts = computed(() => {
  const out: { left: string; removed: boolean }[] = []
  for (const p of parts.value) {
    if (p.added) {
      // gap on left side
      out.push({ left: '', removed: false })
    } else {
      out.push({ left: p.value, removed: !!p.removed })
    }
  }
  return out
})

const rightParts = computed(() => {
  const out: { right: string; added: boolean }[] = []
  for (const p of parts.value) {
    if (p.removed) {
      out.push({ right: '', added: false })
    } else {
      out.push({ right: p.value, added: !!p.added })
    }
  }
  return out
})

const unifiedText = computed(() => {
  return parts.value
    .map((p) => {
      const prefix = p.added ? '+ ' : p.removed ? '- ' : '  '
      return p.value
        .split('\n')
        .filter((l, i, arr) => !(i === arr.length - 1 && l === ''))
        .map((l) => prefix + l)
        .join('\n')
    })
    .join('\n')
})

function escapeHtml(s: string): string {
  return s
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;')
}

function formatLine(raw: string): string {
  // For line-level granularity, just show as-is.
  if (granularity.value === 'line') return escapeHtml(raw)
  // For word/char granularity, mark up within-line changes.
  let out = ''
  for (const p of parts.value) {
    let cls = ''
    if (p.added) cls = 'inner-add'
    else if (p.removed) cls = 'inner-del'
    out += `<span class="${cls}">${escapeHtml(p.value)}</span>`
  }
  return out
}

async function copyDiff() {
  try {
    await navigator.clipboard.writeText(unifiedText.value)
    ElMessage.success(t('common.copySuccess'))
  } catch {
    ElMessage.error(t('common.copyFailed'))
  }
}

function clear() {
  left.value = ''
  right.value = ''
}
</script>

<style scoped lang="scss">
.card-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
}
.hint {
  color: var(--text-secondary);
  font-size: 12px;
  display: inline-flex;
  gap: 6px;
  align-items: center;
}
.split {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 8px;
  font-family: 'JetBrains Mono', Consolas, monospace;
  font-size: 12px;
  max-height: 480px;
  overflow: auto;
}
.pane {
  background: var(--bg-secondary);
  border-radius: 6px;
  overflow: hidden;
}
.line {
  display: flex;
  white-space: pre-wrap;
  word-break: break-all;
  min-height: 18px;
  padding: 1px 0;
  &:hover {
    background: rgba(0, 0, 0, 0.04);
  }
  &.added {
    background: rgba(103, 194, 58, 0.18);
  }
  &.removed {
    background: rgba(245, 108, 108, 0.18);
  }
}
.ln {
  display: inline-block;
  width: 38px;
  text-align: right;
  padding-right: 8px;
  color: var(--text-secondary);
  user-select: none;
  flex-shrink: 0;
}
.content {
  flex: 1;
}
:deep(.inner-add) {
  background: rgba(103, 194, 58, 0.35);
  border-radius: 2px;
}
:deep(.inner-del) {
  background: rgba(245, 108, 108, 0.35);
  border-radius: 2px;
  text-decoration: line-through;
}
.unified {
  background: var(--bg-secondary);
  padding: 12px;
  border-radius: 6px;
  font-family: 'JetBrains Mono', Consolas, monospace;
  font-size: 12px;
  white-space: pre;
  overflow: auto;
  max-height: 480px;
  line-height: 1.4;
}
</style>