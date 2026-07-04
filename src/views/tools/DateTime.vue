<template>
  <div class="page-container">
    <div class="page-header">
      <h2>{{ t('tools.datetime.title') }}</h2>
      <el-button :icon="Refresh" @click="now">{{ t('tools.datetime.now') }}</el-button>
    </div>

    <el-row :gutter="16">
      <el-col :span="12">
        <el-card shadow="never">
          <template #header><strong>{{ t('tools.datetime.unix') }}</strong></template>
          <el-form label-width="120px">
            <el-form-item :label="t('tools.datetime.timestamp')">
              <el-input v-model="ts" @input="fromTs">
                <template #append>{{ tsUnit }}</template>
              </el-input>
            </el-form-item>
            <el-form-item :label="t('tools.datetime.unit')">
              <el-radio-group v-model="tsUnit" @change="fromTs">
                <el-radio-button value="s">{{ t('tools.datetime.seconds') }}</el-radio-button>
                <el-radio-button value="ms">{{ t('tools.datetime.millis') }}</el-radio-button>
              </el-radio-group>
            </el-form-item>
          </el-form>
        </el-card>
      </el-col>

      <el-col :span="12">
        <el-card shadow="never">
          <template #header><strong>{{ t('tools.datetime.iso') }}</strong></template>
          <el-form label-width="120px">
            <el-form-item :label="t('tools.datetime.isoInput')">
              <el-input
                v-model="isoInput"
                :placeholder="t('tools.datetime.isoPlaceholder')"
                @input="fromIso"
              />
            </el-form-item>
            <el-form-item :label="t('tools.datetime.tz')">
              <el-select v-model="tz" filterable @change="recomputeAll" style="width: 100%">
                <el-option
                  v-for="opt in timezones"
                  :key="opt"
                  :label="opt"
                  :value="opt"
                />
              </el-select>
            </el-form-item>
            <el-form-item :label="t('tools.datetime.custom')">
              <el-input
                v-model="format"
                :placeholder="t('tools.datetime.customPlaceholder')"
                @input="recomputeAll"
              />
            </el-form-item>
          </el-form>
        </el-card>
      </el-col>
    </el-row>

    <el-card shadow="never" style="margin-top: 16px">
      <template #header><strong>{{ t('tools.datetime.outputs') }}</strong></template>
      <el-descriptions :column="2" border>
        <el-descriptions-item :label="t('tools.datetime.timestamp') + ' (s)'">{{ outputs.unix_s }}</el-descriptions-item>
        <el-descriptions-item :label="t('tools.datetime.timestamp') + ' (ms)'">{{ outputs.unix_ms }}</el-descriptions-item>
        <el-descriptions-item :label="t('tools.datetime.iso8601')">{{ outputs.iso }}</el-descriptions-item>
        <el-descriptions-item :label="t('tools.datetime.utc')">{{ outputs.utc }}</el-descriptions-item>
        <el-descriptions-item :label="t('tools.datetime.local')">{{ outputs.local }}</el-descriptions-item>
        <el-descriptions-item :label="t('tools.datetime.formatted')">{{ outputs.custom }}</el-descriptions-item>
        <el-descriptions-item :label="t('tools.datetime.dayOfWeek')">{{ outputs.weekday }}</el-descriptions-item>
        <el-descriptions-item :label="t('tools.datetime.weekOfYear')">{{ outputs.week }}</el-descriptions-item>
        <el-descriptions-item :label="t('tools.datetime.dayOfYear')">{{ outputs.dayOfYear }}</el-descriptions-item>
        <el-descriptions-item :label="t('tools.datetime.daysInYear')">{{ outputs.daysInYear }}</el-descriptions-item>
        <el-descriptions-item :label="t('tools.datetime.leapYear')">{{ outputs.leap }}</el-descriptions-item>
        <el-descriptions-item :label="t('tools.datetime.epochDiff')">{{ outputs.diff }}</el-descriptions-item>
      </el-descriptions>
    </el-card>

    <el-card shadow="never" style="margin-top: 16px">
      <template #header><strong>{{ t('tools.datetime.offset') }}</strong></template>
      <el-form label-width="120px">
        <el-form-item :label="t('tools.datetime.op')">
          <el-radio-group v-model="op">
            <el-radio-button value="+">{{ t('tools.datetime.plus') }}</el-radio-button>
            <el-radio-button value="-">{{ t('tools.datetime.minus') }}</el-radio-button>
          </el-radio-group>
        </el-form-item>
        <el-form-item :label="t('tools.datetime.amount')">
          <el-input-number v-model="amount" :min="0" :step="1" />
          <el-select v-model="unit" style="width: 120px; margin-left: 8px">
            <el-option v-for="u in ['seconds','minutes','hours','days','weeks','months','years']" :key="u" :label="u" :value="u" />
          </el-select>
        </el-form-item>
        <el-form-item :label="t('tools.datetime.result')">
          <code>{{ shifted }}</code>
          <el-button text size="small" @click="copyShifted">
            <el-icon><CopyDocument /></el-icon>
            {{ t('common.copy') }}
          </el-button>
        </el-form-item>
      </el-form>
    </el-card>
  </div>
</template>

<script setup lang="ts">
import { reactive, ref, computed, onMounted } from 'vue'
import { useI18n } from 'vue-i18n'
import { ElMessage } from 'element-plus'
import { Refresh, CopyDocument } from '@element-plus/icons-vue'

const { t } = useI18n()

const ts = ref(String(Math.floor(Date.now() / 1000)))
const tsUnit = ref<'s' | 'ms'>('s')
const isoInput = ref('')
const tz = ref(Intl.DateTimeFormat().resolvedOptions().timeZone || 'UTC')
const format = ref('YYYY-MM-DD HH:mm:ss')

const op = ref<'+' | '-'>('+')
const amount = ref(1)
const unit = ref<'seconds' | 'minutes' | 'hours' | 'days' | 'weeks' | 'months' | 'years'>('days')

const date = ref<Date>(new Date())
const timezones = [
  'UTC',
  'Asia/Shanghai',
  'Asia/Tokyo',
  'Asia/Singapore',
  'Asia/Hong_Kong',
  'Europe/London',
  'Europe/Paris',
  'Europe/Berlin',
  'America/New_York',
  'America/Chicago',
  'America/Los_Angeles',
  'Australia/Sydney'
]

function parseDate(d: Date): {
  unix_s: string
  unix_ms: string
  iso: string
  utc: string
  local: string
  custom: string
  weekday: string
  week: string
  dayOfYear: string
  daysInYear: string
  leap: string
  diff: string
} {
  const inTz = new Date(d.toLocaleString('en-US', { timeZone: tz.value }))
  const utc_s = Math.floor(d.getTime() / 1000)
  const utc_ms = d.getTime()
  const isoStr = d.toISOString()
  const utcStr = inTz.toLocaleString('en-US', {
    timeZone: tz.value,
    year: 'numeric',
    month: '2-digit',
    day: '2-digit',
    hour: '2-digit',
    minute: '2-digit',
    second: '2-digit',
    hour12: false
  })
  const localStr = d.toLocaleString()
  const fmtStr = formatDate(d, format.value, tz.value)
  const weekday = new Intl.DateTimeFormat('en-US', { weekday: 'long', timeZone: tz.value }).format(d)
  // ISO week
  const tmp = new Date(Date.UTC(d.getFullYear(), d.getMonth(), d.getDate()))
  const dayNum = tmp.getUTCDay() || 7
  tmp.setUTCDate(tmp.getUTCDate() + 4 - dayNum)
  const yearStart = new Date(Date.UTC(tmp.getUTCFullYear(), 0, 1))
  const week = String(Math.ceil(((tmp.getTime() - yearStart.getTime()) / 86400000 + 1) / 7))
  const start = new Date(d.getFullYear(), 0, 0)
  const dayOfYear = String(Math.floor((d.getTime() - start.getTime()) / 86400000))
  const isLeap = (d.getFullYear() % 4 === 0 && d.getFullYear() % 100 !== 0) || d.getFullYear() % 400 === 0
  const leap = isLeap ? t('common.yes') : t('common.no')
  const daysInYear = isLeap ? '366' : '365'
  const diff = daysBetween(new Date('1970-01-01T00:00:00Z'), d).toFixed(0)
  return {
    unix_s: String(utc_s),
    unix_ms: String(utc_ms),
    iso: isoStr,
    utc: utcStr,
    local: localStr,
    custom: fmtStr,
    weekday,
    week,
    dayOfYear,
    daysInYear,
    leap,
    diff
  }
}

const outputs = reactive({
  unix_s: '',
  unix_ms: '',
  iso: '',
  utc: '',
  local: '',
  custom: '',
  weekday: '',
  week: '',
  dayOfYear: '',
  daysInYear: '',
  leap: '',
  diff: ''
})

const shifted = computed(() => {
  const d = new Date(date.value)
  if (unit.value === 'months') {
    d.setMonth(d.getMonth() + (op.value === '+' ? amount.value : -amount.value))
  } else if (unit.value === 'years') {
    d.setFullYear(d.getFullYear() + (op.value === '+' ? amount.value : -amount.value))
  } else {
    const ms: Record<string, number> = {
      seconds: 1000,
      minutes: 60_000,
      hours: 3_600_000,
      days: 86_400_000,
      weeks: 604_800_000
    }
    d.setTime(d.getTime() + (op.value === '+' ? 1 : -1) * amount.value * ms[unit.value])
  }
  return d.toISOString()
})

function fromTs() {
  const raw = ts.value.trim()
  if (!raw || isNaN(Number(raw))) {
    clearOutputs()
    return
  }
  const n = Number(raw)
  date.value = tsUnit.value === 's' ? new Date(n * 1000) : new Date(n)
  isoInput.value = date.value.toISOString()
  recomputeAll()
}

function fromIso() {
  const raw = isoInput.value.trim()
  if (!raw) {
    clearOutputs()
    return
  }
  const d = new Date(raw)
  if (isNaN(d.getTime())) {
    clearOutputs()
    return
  }
  date.value = d
  ts.value = tsUnit.value === 's' ? String(Math.floor(d.getTime() / 1000)) : String(d.getTime())
  recomputeAll()
}

function now() {
  ts.value = tsUnit.value === 's' ? String(Math.floor(Date.now() / 1000)) : String(Date.now())
  fromTs()
}

function clearOutputs() {
  Object.keys(outputs).forEach((k) => ((outputs as any)[k] = ''))
}

function recomputeAll() {
  const o = parseDate(date.value)
  Object.assign(outputs, o)
}

function daysBetween(a: Date, b: Date): number {
  return (b.getTime() - a.getTime()) / 86400000
}

async function copyShifted() {
  try {
    await navigator.clipboard.writeText(shifted.value)
    ElMessage.success(t('common.copySuccess'))
  } catch {
    ElMessage.error(t('common.copyFailed'))
  }
}

function formatDate(d: Date, fmt: string, tzName: string): string {
  // Use Intl to extract parts in the target timezone, then assemble manually.
  const parts = new Intl.DateTimeFormat('en-US', {
    timeZone: tzName,
    year: 'numeric', month: '2-digit', day: '2-digit',
    hour: '2-digit', minute: '2-digit', second: '2-digit',
    hour12: false, weekday: 'short'
  }).formatToParts(d)
  const get = (t: string) => parts.find((p) => p.type === t)?.value ?? ''
  const map: Record<string, string> = {
    YYYY: get('year'),
    MM: get('month'),
    DD: get('day'),
    HH: get('hour'),
    mm: get('minute'),
    ss: get('second'),
    ddd: get('weekday')
  }
  return fmt.replace(/YYYY|MM|DD|HH|mm|ss|ddd/g, (m) => map[m] ?? m)
}

onMounted(() => {
  now()
})
</script>

<style scoped lang="scss">
code {
  background: var(--bg-secondary);
  padding: 2px 6px;
  border-radius: 4px;
  font-size: 12px;
}
</style>