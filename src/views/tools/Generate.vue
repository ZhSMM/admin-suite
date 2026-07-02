<template>
  <div class="page-container">
    <div class="page-header">
      <h2>{{ t('tools.gen.title') }}</h2>
      <el-radio-group v-model="kind">
        <el-radio-button value="uuid">UUID</el-radio-button>
        <el-radio-button value="password">{{ t('tools.gen.password') }}</el-radio-button>
        <el-radio-button value="passphrase">{{ t('tools.gen.passphrase') }}</el-radio-button>
      </el-radio-group>
    </div>

    <!-- ===== UUID ===== -->
    <template v-if="kind === 'uuid'">
      <el-card shadow="never">
        <el-form label-width="140px">
          <el-form-item :label="t('tools.gen.version')">
            <el-radio-group v-model="uuidVersion">
              <el-radio-button value="v4">v4 (random)</el-radio-button>
              <el-radio-button value="v7">v7 (time-sortable)</el-radio-button>
              <el-radio-button value="nil">nil UUID</el-radio-button>
            </el-radio-group>
          </el-form-item>
          <el-form-item :label="t('tools.gen.uppercase')">
            <el-switch v-model="uppercase" />
          </el-form-item>
          <el-form-item :label="t('tools.gen.hyphens')">
            <el-switch v-model="hyphens" />
          </el-form-item>
          <el-form-item :label="t('tools.gen.braces')">
            <el-switch v-model="braces" />
          </el-form-item>
          <el-form-item :label="t('tools.gen.count')">
            <el-input-number v-model="count" :min="1" :max="200" />
          </el-form-item>
          <el-form-item>
            <el-button type="primary" :icon="Refresh" @click="generateUuid">
              {{ t('tools.gen.generate') }}
            </el-button>
            <el-button :icon="CopyDocument" @click="copyAll(uuids)">
              {{ t('common.copy') }}
            </el-button>
          </el-form-item>
        </el-form>
      </el-card>
      <el-card shadow="never" style="margin-top: 16px">
        <template #header>
          <div class="card-header">
            <strong>{{ t('tools.gen.output') }}</strong>
            <span class="hint">{{ uuids.length }} UUID</span>
          </div>
        </template>
        <el-input
          v-model="uuidText"
          type="textarea"
          :rows="10"
          readonly
          :placeholder="t('tools.gen.clickGenerate')"
        />
      </el-card>
    </template>

    <!-- ===== Password ===== -->
    <template v-else-if="kind === 'password'">
      <el-card shadow="never">
        <el-form label-width="180px">
          <el-form-item :label="t('tools.gen.length')">
            <el-input-number v-model="pwLength" :min="4" :max="128" />
          </el-form-item>
          <el-form-item :label="t('tools.gen.uppercase')">
            <el-switch v-model="pwUpper" />
          </el-form-item>
          <el-form-item :label="t('tools.gen.lowercase')">
            <el-switch v-model="pwLower" />
          </el-form-item>
          <el-form-item :label="t('tools.gen.digits')">
            <el-switch v-model="pwDigits" />
          </el-form-item>
          <el-form-item :label="t('tools.gen.symbols')">
            <el-switch v-model="pwSymbols" />
          </el-form-item>
          <el-form-item :label="t('tools.gen.exclude')">
            <el-input v-model="pwExclude" :placeholder="t('tools.gen.excludePlaceholder')" />
          </el-form-item>
          <el-form-item :label="t('tools.gen.count')">
            <el-input-number v-model="count" :min="1" :max="100" />
          </el-form-item>
          <el-form-item>
            <el-button type="primary" :icon="Refresh" @click="generatePassword">
              {{ t('tools.gen.generate') }}
            </el-button>
            <el-button :icon="CopyDocument" @click="copyAll(passwords)">
              {{ t('common.copy') }}
            </el-button>
          </el-form-item>
        </el-form>
      </el-card>
      <el-card shadow="never" style="margin-top: 16px">
        <template #header>
          <div class="card-header">
            <strong>{{ t('tools.gen.output') }}</strong>
            <span class="hint">
              {{ t('tools.gen.strength') }}: <el-tag :type="strengthTag as any">{{ strength }}</el-tag>
            </span>
          </div>
        </template>
        <el-input
          v-model="passwordText"
          type="textarea"
          :rows="10"
          readonly
          :placeholder="t('tools.gen.clickGenerate')"
        />
      </el-card>
    </template>

    <!-- ===== Passphrase (Diceware-ish) ===== -->
    <template v-else>
      <el-card shadow="never">
        <el-form label-width="180px">
          <el-form-item :label="t('tools.gen.wordCount')">
            <el-input-number v-model="phraseWords" :min="2" :max="20" />
          </el-form-item>
          <el-form-item :label="t('tools.gen.separator')">
            <el-input v-model="phraseSep" style="width: 120px" />
          </el-form-item>
          <el-form-item :label="t('tools.gen.capitalize')">
            <el-switch v-model="phraseCap" />
          </el-form-item>
          <el-form-item :label="t('tools.gen.appendDigits')">
            <el-switch v-model="phraseDigits" />
          </el-form-item>
          <el-form-item>
            <el-button type="primary" :icon="Refresh" @click="generatePassphrase">
              {{ t('tools.gen.generate') }}
            </el-button>
            <el-button :icon="CopyDocument" @click="copyAll(passphrases)">
              {{ t('common.copy') }}
            </el-button>
          </el-form-item>
        </el-form>
      </el-card>
      <el-card shadow="never" style="margin-top: 16px">
        <template #header><strong>{{ t('tools.gen.output') }}</strong></template>
        <el-input
          v-model="passphraseText"
          type="textarea"
          :rows="6"
          readonly
          :placeholder="t('tools.gen.clickGenerate')"
        />
      </el-card>
    </template>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted } from 'vue'
import { useI18n } from 'vue-i18n'
import { ElMessage } from 'element-plus'
import { Refresh, CopyDocument } from '@element-plus/icons-vue'

const { t } = useI18n()

const kind = ref<'uuid' | 'password' | 'passphrase'>('uuid')

// ---- UUID ----
const uuidVersion = ref<'v4' | 'v7' | 'nil'>('v4')
const uppercase = ref(false)
const hyphens = ref(true)
const braces = ref(false)
const count = ref(10)

const uuids = ref<string[]>([])
const uuidText = computed(() => uuids.value.join('\n'))

function makeUuidV4(): string {
  if (typeof crypto.randomUUID === 'function') return crypto.randomUUID()
  // Fallback
  const bytes = crypto.getRandomValues(new Uint8Array(16))
  bytes[6] = (bytes[6] & 0x0f) | 0x40
  bytes[8] = (bytes[8] & 0x3f) | 0x80
  const hex = Array.from(bytes, (b) => b.toString(16).padStart(2, '0')).join('')
  return `${hex.slice(0,8)}-${hex.slice(8,12)}-${hex.slice(12,16)}-${hex.slice(16,20)}-${hex.slice(20)}`
}

function makeUuidV7(): string {
  const ts = Date.now()
  const tsHex = ts.toString(16).padStart(12, '0') // 48-bit timestamp
  const rand = crypto.getRandomValues(new Uint8Array(10))
  // Set version (7) in byte 6 high nibble, variant (10) in byte 8 high bits.
  rand[0] = (rand[0] & 0x0f) | 0x70
  rand[2] = (rand[2] & 0x3f) | 0x80
  const randHex = Array.from(rand, (b) => b.toString(16).padStart(2, '0')).join('')
  return `${tsHex.slice(0,8)}-${tsHex.slice(8,12)}-7${randHex.slice(0,3)}-${randHex.slice(2,6)}-${randHex.slice(4)}`
}

function formatUuid(u: string): string {
  let out = u
  if (!hyphens.value) out = out.replace(/-/g, '')
  if (uppercase.value) out = out.toUpperCase()
  if (braces.value) out = `{${out}}`
  return out
}

function generateUuid() {
  const list: string[] = []
  for (let i = 0; i < count.value; i++) {
    let raw: string
    if (uuidVersion.value === 'v4') raw = makeUuidV4()
    else if (uuidVersion.value === 'v7') raw = makeUuidV7()
    else raw = '00000000-0000-0000-0000-000000000000'
    list.push(formatUuid(raw))
  }
  uuids.value = list
}

// ---- Password ----
const pwLength = ref(16)
const pwUpper = ref(true)
const pwLower = ref(true)
const pwDigits = ref(true)
const pwSymbols = ref(true)
const pwExclude = ref('')
const passwords = ref<string[]>([])
const passwordText = computed(() => passwords.value.join('\n'))

const UPPER = 'ABCDEFGHIJKLMNOPQRSTUVWXYZ'
const LOWER = 'abcdefghijklmnopqrstuvwxyz'
const DIGITS = '0123456789'
const SYMBOLS = '!@#$%^&*()-_=+[]{};:,.<>/?'

function secureIndex(max: number): number {
  const bytes = crypto.getRandomValues(new Uint8Array(max * 2))
  let idx = 0
  let last = 0
  for (let i = 0; i < bytes.length; i++) {
    last = (last + bytes[i]) % 256
    idx = (idx * 256 + last) % max
  }
  return idx
}

function generatePassword() {
  const pool: string[] = []
  if (pwUpper.value) pool.push(UPPER)
  if (pwLower.value) pool.push(LOWER)
  if (pwDigits.value) pool.push(DIGITS)
  if (pwSymbols.value) pool.push(SYMBOLS)
  if (pool.length === 0) {
    ElMessage.warning(t('tools.gen.enableOne'))
    return
  }
  const excluded = new Set(pwExclude.value.split(''))
  const allChars = pool.join('').split('').filter((c) => !excluded.has(c))
  const requiredChars = pool
    .map((s) => s.split('').find((c) => !excluded.has(c)))
    .filter((c): c is string => !!c)
  const list: string[] = []
  for (let i = 0; i < count.value; i++) {
    const buf: string[] = [...requiredChars]
    while (buf.length < pwLength.value) {
      buf.push(allChars[secureIndex(allChars.length)])
    }
    // Shuffle with crypto randomness.
    for (let j = buf.length - 1; j > 0; j--) {
      const k = secureIndex(j + 1)
      ;[buf[j], buf[k]] = [buf[k], buf[j]]
    }
    list.push(buf.slice(0, pwLength.value).join(''))
  }
  passwords.value = list
}

const strength = computed(() => {
  if (!passwords.value.length) return ''
  const p = passwords.value[0]
  let s = 0
  if (/[A-Z]/.test(p)) s++
  if (/[a-z]/.test(p)) s++
  if (/[0-9]/.test(p)) s++
  if (/[^A-Za-z0-9]/.test(p)) s++
  if (p.length >= 16) s++
  if (s >= 5) return t('tools.gen.strong')
  if (s >= 3) return t('tools.gen.medium')
  return t('tools.gen.weak')
})
const strengthTag = computed(() => {
  if (strength.value === t('tools.gen.strong')) return 'success'
  if (strength.value === t('tools.gen.medium')) return 'warning'
  return 'danger'
})

// ---- Passphrase (Diceware-ish) ----
const phraseWords = ref(5)
const phraseSep = ref('-')
const phraseCap = ref(true)
const phraseDigits = ref(false)
const passphrases = ref<string[]>([])
const passphraseText = computed(() => passphrases.value.join('\n'))

// 256 words from EFF short wordlist, sufficient for casual use.
const WORDS = [
  'acid','aged','also','area','army','away','baby','back','ball','band',
  'bank','base','bath','bear','beat','been','beer','bell','belt','best',
  'bike','bill','bird','blue','boat','body','bomb','bone','book','born',
  'boss','both','burn','busy','cake','call','calm','camp','card','care',
  'case','cash','cast','cell','chat','chin','chip','city','club','coal',
  'coat','code','cold','come','cook','cool','cope','copy','core','cost',
  'crew','crop','dark','data','date','dawn','dead','deal','dear','debt',
  'deep','deny','desk','dial','dice','diet','dirt','disc','dish','disk',
  'dock','does','done','door','dose','down','draw','drew','drop','drug',
  'drum','dual','duke','dust','duty','each','earn','ease','east','easy',
  'edge','else','even','ever','evil','exam','exec','exit','face','fact',
  'fail','fair','fall','farm','fast','fate','fear','feed','feel','feet',
  'fell','felt','file','fill','film','find','fine','fire','firm','fish',
  'flag','flat','flew','flip','flow','foam','fold','folk','fond','food',
  'fool','foot','ford','form','fort','four','free','from','fuel','full',
  'fund','gain','game','gang','gate','gave','gear','gene','gift','girl',
  'give','glad','goal','goes','gold','golf','gone','good','gray','grew',
  'grid','grip','grow','gulf','hack','hair','half','hall','hand','hang',
  'hard','harm','hate','have','head','hear','heat','held','hell','help',
  'here','hero','high','hill','hint','hire','hold','hole','holy','home',
  'hope','host','hour','huge','hung','hunt','hurt','icon','idea','inch',
  'into','iron','item','jack','jane','jazz','jean','john','join','joke',
  'jump','jury','just','keen','keep','kept','kick','kill','kind','king',
  'knee','knew','know','lack','laid','lake','lamp','land','lane','last',
  'late','lazy','lead','left','lend','less','life','lift','like','line',
  'link','list','live','load','loan','lock','loft','logo','long','look',
  'lord','lose','loss','lost','loud','love','luck','made','mail','main',
  'make','male','many','mark','mask','mass','matt','mayo','meal','mean',
  'meat','meet','mile','milk','mind','mine','miss','mode','mood','moon',
  'more','most','move','much','must','myth','name','navy','near','neat',
  'neck','need','news','nice','node','none','norm','nose','note','noun',
  'odds','okay','once','only','onto','open','over','pace','pack','page',
  'paid','pain','pair','palm','park','part','pass','past','path','peak',
  'peer','pick','pine','pink','pipe','plan','play','plot','plug','plus',
  'poem','poet','poll','pond','pool','poor','pope','post','pour','pray',
  'pull','pump','pure','push','quit','race','rain','rank','rare','rate',
  'read','real','rear','rely','rent','rest','rice','rich','ride','ring',
  'rise','risk','road','rock','rode','role','roll','roof','room','root',
  'rope','rose','ruin','rule','rush','safe','sail','sake','sale','salt',
  'same','sand','sang','save','seal','seat','seed','seek','seem','seen',
  'self','sell','send','sent','sept','ship','shop','shot','show','shut',
  'sick','side','sign','site','size','skin','slip','slow','snow','soft',
  'soil','sold','sole','some','song','soon','sort','soul','spot','star',
  'stay','stem','step','stop','such','suit','sure','swim','tail','take',
  'tale','talk','tall','tank','tape','task','taxi','team','tell','temp',
  'tend','term','test','text','than','that','them','then','they','thin',
  'this','thus','tide','till','time','tiny','tire','told','toll','tone',
  'took','tool','tops','tore','torn','tour','town','trap','tree','trim',
  'trio','trip','true','tube','tuck','tune','turn','twin','type','ugly',
  'undo','unit','unto','upon','urge','used','user','vast','vice','view',
  'vote','wage','wait','wake','walk','wall','want','ward','warm','warn',
  'wash','wave','ways','weak','wear','week','well','went','were','west',
  'what','when','whom','wide','wife','wild','will','wind','wine','wing',
  'wire','wise','wish','with','wood','word','wore','work','worn','wrap',
  'yard','year','your','zero','zone'
]

function generatePassphrase() {
  const list: string[] = []
  for (let i = 0; i < count.value; i++) {
    const words: string[] = []
    for (let j = 0; j < phraseWords.value; j++) {
      const w = WORDS[secureIndex(WORDS.length)]
      words.push(phraseCap.value ? w[0].toUpperCase() + w.slice(1) : w)
    }
    let phrase = words.join(phraseSep.value)
    if (phraseDigits.value) {
      phrase += phraseSep.value + String(secureIndex(100))
    }
    list.push(phrase)
  }
  passphrases.value = list
}

async function copyAll(values: string[]) {
  if (!values.length) return
  try {
    await navigator.clipboard.writeText(values.join('\n'))
    ElMessage.success(t('common.copySuccess'))
  } catch {
    ElMessage.error(t('common.copyFailed'))
  }
}

onMounted(() => {
  generateUuid()
})
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
}
</style>