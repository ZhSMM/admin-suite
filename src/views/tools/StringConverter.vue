<template>
  <div class="page-container">
    <div class="page-header">
      <h2>{{ t('tools.string.title') }}</h2>
      <el-button @click="swap">
        <el-icon><Refresh /></el-icon>
        {{ t('tools.string.swap') }}
      </el-button>
    </div>

    <el-tabs v-model="mode" type="border-card">
      <!-- ============ Unicode ============ -->
      <el-tab-pane :label="t('tools.string.tabUnicode')" name="unicode">
        <el-form label-width="180px">
          <el-form-item :label="t('tools.string.format')">
            <el-radio-group v-model="unicodeFormat">
              <el-radio value="escape">{{ t('tools.string.unicodeEscape') }}</el-radio>
              <el-radio value="hex">{{ t('tools.string.unicodeHex') }}</el-radio>
              <el-radio value="code">{{ t('tools.string.unicodeCode') }}</el-radio>
              <el-radio value="decimal">{{ t('tools.string.unicodeDecimal') }}</el-radio>
              <el-radio value="raw">{{ t('tools.string.unicodeRaw') }}</el-radio>
            </el-radio-group>
          </el-form-item>
        </el-form>
      </el-tab-pane>

      <!-- ============ HTML ============ -->
      <el-tab-pane :label="t('tools.string.tabHtml')" name="html">
        <el-form label-width="180px">
          <el-form-item :label="t('tools.string.format')">
            <el-radio-group v-model="htmlFormat">
              <el-radio value="numeric">{{ t('tools.string.htmlNumeric') }}</el-radio>
              <el-radio value="hex">{{ t('tools.string.htmlHex') }}</el-radio>
              <el-radio value="named">{{ t('tools.string.htmlNamed') }}</el-radio>
              <el-radio value="all">{{ t('tools.string.htmlAll') }}</el-radio>
            </el-radio-group>
          </el-form-item>
          <el-form-item :label="t('tools.string.direction')">
            <el-radio-group v-model="htmlDirection">
              <el-radio value="encode">{{ t('tools.string.encode') }}</el-radio>
              <el-radio value="decode">{{ t('tools.string.decode') }}</el-radio>
            </el-radio-group>
          </el-form-item>
        </el-form>
      </el-tab-pane>

      <!-- ============ URL ============ -->
      <el-tab-pane :label="t('tools.string.tabUrl')" name="url">
        <el-form label-width="180px">
          <el-form-item :label="t('tools.string.format')">
            <el-radio-group v-model="urlMode">
              <el-radio value="component">encodeURIComponent</el-radio>
              <el-radio value="uri">encodeURI</el-radio>
              <el-radio value="rfc3986">RFC 3986</el-radio>
            </el-radio-group>
          </el-form-item>
          <el-form-item :label="t('tools.string.direction')">
            <el-radio-group v-model="urlDirection">
              <el-radio value="encode">{{ t('tools.string.encode') }}</el-radio>
              <el-radio value="decode">{{ t('tools.string.decode') }}</el-radio>
            </el-radio-group>
          </el-form-item>
        </el-form>
      </el-tab-pane>

      <!-- ============ Normalize ============ -->
      <el-tab-pane :label="t('tools.string.tabNormalize')" name="normalize">
        <el-form label-width="180px">
          <el-form-item :label="t('tools.string.form')">
            <el-radio-group v-model="normForm">
              <el-radio value="NFC">NFC</el-radio>
              <el-radio value="NFD">NFD</el-radio>
              <el-radio value="NFKC">NFKC</el-radio>
              <el-radio value="NFKD">NFKD</el-radio>
            </el-radio-group>
          </el-form-item>
          <el-form-item>
            <small style="color: var(--text-secondary)">{{ t('tools.string.normalizeHelp') }}</small>
          </el-form-item>
        </el-form>
      </el-tab-pane>

      <!-- ============ ASCII ============ -->
      <el-tab-pane :label="t('tools.string.tabAscii')" name="ascii">
        <el-form label-width="180px">
          <el-form-item :label="t('tools.string.mode')">
            <el-radio-group v-model="asciiMode">
              <el-radio value="strip">{{ t('tools.string.asciiStrip') }}</el-radio>
              <el-radio value="replace">{{ t('tools.string.asciiReplace') }}</el-radio>
              <el-radio value="escape">{{ t('tools.string.asciiEscape') }}</el-radio>
              <el-radio value="codepoints">{{ t('tools.string.asciiCodepoints') }}</el-radio>
            </el-radio-group>
          </el-form-item>
        </el-form>
      </el-tab-pane>

      <!-- ============ JS / JSON / CSS ============ -->
      <el-tab-pane :label="t('tools.string.tabString')" name="js">
        <el-form label-width="180px">
          <el-form-item :label="t('tools.string.format')">
            <el-radio-group v-model="stringFormat">
              <el-radio value="json">{{ t('tools.string.sJson') }}</el-radio>
              <el-radio value="js">{{ t('tools.string.sJs') }}</el-radio>
              <el-radio value="css">{{ t('tools.string.sCss') }}</el-radio>
            </el-radio-group>
          </el-form-item>
          <el-form-item :label="t('tools.string.direction')">
            <el-radio-group v-model="stringDirection">
              <el-radio value="encode">{{ t('tools.string.encode') }}</el-radio>
              <el-radio value="decode">{{ t('tools.string.decode') }}</el-radio>
            </el-radio-group>
          </el-form-item>
        </el-form>
      </el-tab-pane>
    </el-tabs>

    <el-row :gutter="12" style="margin-top: 12px">
      <el-col :span="12">
        <el-card shadow="never">
          <template #header>
            <strong>{{ t('tools.string.input') }}</strong>
          </template>
          <el-input
            v-model="input"
            type="textarea"
            :rows="14"
            spellcheck="false"
            :placeholder="t('tools.string.placeholder')"
            @input="compute"
          />
        </el-card>
      </el-col>
      <el-col :span="12">
        <el-card shadow="never">
          <template #header>
            <div class="card-header">
              <strong>{{ t('tools.string.output') }}</strong>
              <el-button text size="small" :icon="DocumentCopy" @click="copy(output)">
                {{ t('common.copy') }}
              </el-button>
            </div>
          </template>
          <el-input
            v-model="output"
            type="textarea"
            :rows="14"
            spellcheck="false"
            readonly
          />
          <div v-if="error" class="err">
            <el-icon><CircleClose /></el-icon> {{ error }}
          </div>
        </el-card>
      </el-col>
    </el-row>
  </div>
</template>

<script setup lang="ts">
import { ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { ElMessage } from 'element-plus'
import { DocumentCopy, Refresh, CircleClose } from '@element-plus/icons-vue'

const { t } = useI18n()

const mode = ref<'unicode' | 'html' | 'url' | 'normalize' | 'ascii' | 'js'>('unicode')
const input = ref('你好,world! \u00e9 \u4e2d\u6587 — "quotes" & <tags>')
const output = ref('')
const error = ref('')

// ---- Unicode tab ----
const unicodeFormat = ref<'escape' | 'hex' | 'code' | 'decimal' | 'raw'>('escape')

// ---- HTML tab ----
const htmlFormat = ref<'numeric' | 'hex' | 'named' | 'all'>('named')
const htmlDirection = ref<'encode' | 'decode'>('encode')

// ---- URL tab ----
const urlMode = ref<'component' | 'uri' | 'rfc3986'>('component')
const urlDirection = ref<'encode' | 'decode'>('encode')

// ---- Normalize tab ----
const normForm = ref<'NFC' | 'NFD' | 'NFKC' | 'NFKD'>('NFC')

// ---- ASCII tab ----
const asciiMode = ref<'strip' | 'replace' | 'escape' | 'codepoints'>('escape')

// ---- JS / JSON / CSS tab ----
const stringFormat = ref<'json' | 'js' | 'css'>('json')
const stringDirection = ref<'encode' | 'decode'>('encode')

// Watch everything; recompute on any change.
watch([mode, input, unicodeFormat, htmlFormat, htmlDirection, urlMode, urlDirection, normForm, asciiMode, stringFormat, stringDirection], compute, { immediate: true })

function compute() {
  error.value = ''
  if (!input.value) {
    output.value = ''
    return
  }
  try {
    switch (mode.value) {
      case 'unicode': output.value = computeUnicode(); break
      case 'html': output.value = computeHtml(); break
      case 'url': output.value = computeUrl(); break
      case 'normalize': output.value = input.value.normalize(normForm.value); break
      case 'ascii': output.value = computeAscii(); break
      case 'js': output.value = computeString(); break
    }
  } catch (e: any) {
    error.value = e.message
    output.value = ''
  }
}

// ============ Per-mode implementations ============

function computeUnicode(): string {
  const s = input.value
  switch (unicodeFormat.value) {
    case 'escape':
      return Array.from(s).map((c) => {
        const code = c.charCodeAt(0)
        if (code < 128) return c
        if (code < 0x10000) return '\\u' + code.toString(16).padStart(4, '0')
        // Surrogate pair
        const hi = Math.floor((code - 0x10000) / 0x400) + 0xd800
        const lo = ((code - 0x10000) % 0x400) + 0xdc00
        return '\\u' + hi.toString(16).padStart(4, '0') + '\\u' + lo.toString(16).padStart(4, '0')
      }).join('')
    case 'hex':
      return Array.from(s).map((c) => '\\x' + c.charCodeAt(0).toString(16).padStart(2, '0')).join('')
    case 'code':
      return Array.from(s).map((c) => 'U+' + c.charCodeAt(0).toString(16).toUpperCase().padStart(4, '0')).join(' ')
    case 'decimal':
      return Array.from(s).map((c) => '&#' + c.charCodeAt(0) + ';').join('')
    case 'raw':
      return s
  }
}

const HTML_NAMED: Record<string, string> = {
  '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&apos;',
  '`': '&grave;', ' ': '&nbsp;', '¡': '&iexcl;', '¢': '&cent;', '£': '&pound;',
  '©': '&copy;', '®': '&reg;', '°': '&deg;', '±': '&plusmn;', '×': '&times;',
  '÷': '&divide;', '§': '&sect;', '¶': '&para;', '·': '&middot;', '‘': '&lsquo;',
  '’': '&rsquo;', '“': '&ldquo;', '”': '&rdquo;', '€': '&euro;', '¥': '&yen;',
  '←': '&larr;', '→': '&rarr;', '↑': '&uarr;', '↓': '&darr;', '↔': '&harr;',
  '♠': '&spades;', '♣': '&clubs;', '♥': '&hearts;', '♦': '&diams;',
  '∞': '&infin;', '∂': '&part;', '∇': '&nabla;', '∑': '&sum;', '∏': '&prod;',
  '∫': '&int;', '≈': '&asymp;', '≠': '&ne;', '≤': '&le;', '≥': '&ge;'
}

const HTML_NAMED_REVERSE: Record<string, string> = Object.fromEntries(
  Object.entries(HTML_NAMED).map(([k, v]) => [v, k])
)

function escapeHtmlAll(s: string): string {
  let out = ''
  for (const c of s) {
    if (HTML_NAMED[c]) {
      out += HTML_NAMED[c]
    } else {
      const code = c.charCodeAt(0)
      out += '&#' + code + ';'
    }
  }
  return out
}

function escapeHtmlNamed(s: string): string {
  let out = ''
  for (const c of s) {
    if (HTML_NAMED[c]) {
      out += HTML_NAMED[c]
    } else {
      out += c  // leave non-named as-is
    }
  }
  return out
}

function escapeHtmlNumeric(s: string): string {
  let out = ''
  for (const c of s) {
    const code = c.charCodeAt(0)
    out += '&#' + code + ';'
  }
  return out
}

function escapeHtmlHex(s: string): string {
  let out = ''
  for (const c of s) {
    const code = c.charCodeAt(0)
    out += '&#x' + code.toString(16) + ';'
  }
  return out
}

function decodeHtmlEntities(s: string): string {
  // Decode named first, then numeric, then hex.
  // Named entities (only the most common ones; full lookup would need a library).
  let out = s
  for (const [ent, ch] of Object.entries(HTML_NAMED_REVERSE)) {
    out = out.split(ent).join(ch)
  }
  out = out.replace(/&#x([0-9a-fA-F]+);/g, (_, h) => String.fromCodePoint(parseInt(h, 16)))
  out = out.replace(/&#(\d+);/g, (_, n) => String.fromCodePoint(parseInt(n, 10)))
  return out
}

function computeHtml(): string {
  if (htmlDirection.value === 'decode') {
    return decodeHtmlEntities(input.value)
  }
  switch (htmlFormat.value) {
    case 'numeric': return escapeHtmlNumeric(input.value)
    case 'hex': return escapeHtmlHex(input.value)
    case 'named': return escapeHtmlNamed(input.value)
    case 'all': return escapeHtmlAll(input.value)
  }
}

function computeUrl(): string {
  const s = input.value
  if (urlDirection.value === 'decode') {
    try {
      if (urlMode.value === 'uri') return decodeURI(s)
      return decodeURIComponent(s)
    } catch (e: any) {
      throw new Error(t('tools.string.urlDecodeError') + ': ' + e.message)
    }
  }
  if (urlMode.value === 'rfc3986') {
    // RFC 3986: encodeURIComponent then re-encode characters that RFC 3986
    // reserves but encodeURIComponent doesn't.
    return encodeURIComponent(s)
      .replace(/[!'()*]/g, (c) => '%' + c.charCodeAt(0).toString(16).toUpperCase())
  }
  if (urlMode.value === 'uri') return encodeURI(s)
  return encodeURIComponent(s)
}

function computeAscii(): string {
  const s = input.value
  switch (asciiMode.value) {
    case 'strip':
      // Strip combining marks too (U+0300..U+036F) for cleaner output.
      return s
        .normalize('NFD')
        .replace(/[\u0300-\u036f]/g, '')
        .replace(/[^\x00-\x7f]/g, '')
    case 'replace':
      return s.replace(/[^\x00-\x7f]/g, '?')
    case 'escape':
      return Array.from(s)
        .map((c) => {
          const code = c.charCodeAt(0)
          if (code < 128) return c
          return '\\u' + (code > 0xffff ? '...' : code.toString(16).padStart(4, '0'))
        })
        .join('')
    case 'codepoints':
      return Array.from(s)
        .map((c) => {
          const code = c.charCodeAt(0)
          return code < 128 ? `${c} (0x${code.toString(16)})` : `${c} (U+${code.toString(16).toUpperCase().padStart(4, '0')})`
        })
        .join(' ')
  }
}

function escapeJsonString(s: string): string {
  return JSON.stringify(s)
}

function escapeJsString(s: string): string {
  return "'" + s.replace(/\\/g, '\\\\').replace(/'/g, "\\'").replace(/\n/g, '\\n').replace(/\r/g, '\\r').replace(/\t/g, '\\t') + "'"
}

function escapeCssString(s: string): string {
  return "'" + s.replace(/\\/g, '\\\\').replace(/'/g, "\\'").replace(/\n/g, '\\A ').replace(/\r/g, '\\D ') + "'"
}

function decodeJsonString(s: string): string {
  // Allow user to paste with or without surrounding quotes.
  const trimmed = s.trim()
  const payload = trimmed.startsWith('"') && trimmed.endsWith('"') ? trimmed : JSON.stringify(trimmed)
  return JSON.parse(payload) as string
}

function decodeJsString(s: string): string {
  // Very small JS string literal decoder: \\, \', \n, \r, \t, \uXXXX, \xXX.
  return s.replace(/^'|'$/g, '')
    .replace(/\\'/g, "'")
    .replace(/\\\\/g, '\\')
    .replace(/\\n/g, '\n')
    .replace(/\\r/g, '\r')
    .replace(/\\t/g, '\t')
    .replace(/\\u([0-9a-fA-F]{4})/g, (_, h) => String.fromCharCode(parseInt(h, 16)))
    .replace(/\\x([0-9a-fA-F]{2})/g, (_, h) => String.fromCharCode(parseInt(h, 16)))
}

function decodeCssString(s: string): string {
  // CSS allows \HH (hex) at the end of escapes, no \n/\r escapes.  Just decode hex + quotes.
  return s
    .replace(/^'|'$/g, '')
    .replace(/\\([0-9a-fA-F]{1,6})\s?/g, (m, h) => {
      try { return String.fromCodePoint(parseInt(h, 16)) } catch { return m }
    })
    .replace(/\\([\\'"])/g, '$1')
}

function computeString(): string {
  if (stringDirection.value === 'decode') {
    switch (stringFormat.value) {
      case 'json': return decodeJsonString(input.value)
      case 'js': return decodeJsString(input.value)
      case 'css': return decodeCssString(input.value)
    }
  }
  switch (stringFormat.value) {
    case 'json': return escapeJsonString(input.value)
    case 'js': return escapeJsString(input.value)
    case 'css': return escapeCssString(input.value)
  }
}

function swap() {
  const tmp = input.value
  input.value = output.value
  // Flip direction so swap actually inverts.
  if (mode.value === 'html') htmlDirection.value = htmlDirection.value === 'encode' ? 'decode' : 'encode'
  if (mode.value === 'url') urlDirection.value = urlDirection.value === 'encode' ? 'decode' : 'encode'
  if (mode.value === 'js') stringDirection.value = stringDirection.value === 'encode' ? 'decode' : 'encode'
  // Re-compute with the new input.
  compute()
  // Restore swap effect on visible output too.
  output.value = tmp
}

async function copy(value: string) {
  if (!value) return
  try {
    await navigator.clipboard.writeText(value)
    ElMessage.success(t('common.copySuccess'))
  } catch {
    ElMessage.error(t('common.copyFailed'))
  }
}
</script>

<style scoped lang="scss">
.card-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
}
.err {
  margin-top: 8px;
  color: var(--danger-color);
  display: flex;
  align-items: center;
  gap: 4px;
  font-size: 13px;
}
</style>