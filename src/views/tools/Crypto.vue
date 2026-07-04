<template>
  <div class="page-container">
    <div class="page-header">
      <h2>{{ t('tools.crypto.title') }}</h2>
    </div>

    <el-tabs v-model="algo" type="border-card">
      <!-- ============ AES-GCM (modern) ============ -->
      <el-tab-pane :label="t('tools.crypto.aesGcm')" name="aes-gcm">
        <el-row :gutter="12">
          <el-col :span="12">
            <el-form label-width="120px">
              <el-form-item :label="t('tools.crypto.key')">
                <el-input v-model="aesKey" :placeholder="t('tools.crypto.aesKeyPlaceholder')" />
                <div style="margin-top: 4px">
                  <el-button size="small" @click="randomAesKey">{{ t('tools.crypto.randomKey') }}</el-button>
                </div>
              </el-form-item>
              <el-form-item :label="t('tools.crypto.iv')">
                <el-input v-model="aesIv" :placeholder="t('tools.crypto.aesGcmIvPlaceholder')" />
                <div style="margin-top: 4px">
                  <el-button size="small" @click="randomGcmIv">{{ t('tools.crypto.randomIv') }}</el-button>
                </div>
              </el-form-item>
              <el-form-item :label="t('tools.crypto.aad')">
                <el-input v-model="aesAad" :placeholder="t('tools.crypto.aadPlaceholder')" />
              </el-form-item>
              <el-form-item>
                <el-space>
                  <el-button type="primary" :icon="Lock" @click="aesGcmEncrypt">{{ t('tools.crypto.encrypt') }}</el-button>
                  <el-button :icon="Unlock" @click="aesGcmDecrypt">{{ t('tools.crypto.decrypt') }}</el-button>
                </el-space>
              </el-form-item>
            </el-form>
          </el-col>
          <el-col :span="12">
            <el-form label-width="120px">
              <el-form-item :label="t('tools.crypto.plaintext')">
                <el-input v-model="aesPlain" type="textarea" :rows="5" />
              </el-form-item>
              <el-form-item :label="t('tools.crypto.ciphertext')">
                <el-input v-model="aesCipher" type="textarea" :rows="5" readonly />
                <el-button size="small" text :icon="DocumentCopy" @click="copy(aesCipher)">{{ t('common.copy') }}</el-button>
              </el-form-item>
            </el-form>
          </el-col>
        </el-row>
      </el-tab-pane>

      <!-- ============ AES-CBC ============ -->
      <el-tab-pane :label="t('tools.crypto.aesCbc')" name="aes-cbc">
        <el-row :gutter="12">
          <el-col :span="12">
            <el-form label-width="120px">
              <el-form-item :label="t('tools.crypto.key')">
                <el-input v-model="cbcKey" :placeholder="t('tools.crypto.aesKeyPlaceholder')" />
                <el-button size="small" @click="randomCbcKey">{{ t('tools.crypto.randomKey') }}</el-button>
              </el-form-item>
              <el-form-item :label="t('tools.crypto.iv')">
                <el-input v-model="cbcIv" :placeholder="t('tools.crypto.aesCbcIvPlaceholder')" />
                <el-button size="small" @click="randomCbcIv">{{ t('tools.crypto.randomIv') }}</el-button>
              </el-form-item>
              <el-form-item>
                <el-space>
                  <el-button type="primary" :icon="Lock" @click="aesCbcEncrypt">{{ t('tools.crypto.encrypt') }}</el-button>
                  <el-button :icon="Unlock" @click="aesCbcDecrypt">{{ t('tools.crypto.decrypt') }}</el-button>
                </el-space>
              </el-form-item>
            </el-form>
          </el-col>
          <el-col :span="12">
            <el-form label-width="120px">
              <el-form-item :label="t('tools.crypto.plaintext')">
                <el-input v-model="cbcPlain" type="textarea" :rows="5" />
              </el-form-item>
              <el-form-item :label="t('tools.crypto.ciphertext')">
                <el-input v-model="cbcCipher" type="textarea" :rows="5" readonly />
                <el-button size="small" text :icon="DocumentCopy" @click="copy(cbcCipher)">{{ t('common.copy') }}</el-button>
              </el-form-item>
            </el-form>
          </el-col>
        </el-row>
      </el-tab-pane>

      <!-- ============ RSA ============ -->
      <el-tab-pane :label="t('tools.crypto.rsa')" name="rsa">
        <el-form label-width="160px">
          <el-form-item :label="t('tools.crypto.rsaKeySize')">
            <el-radio-group v-model="rsaSize">
              <el-radio :value="2048">2048</el-radio>
              <el-radio :value="3072">3072</el-radio>
              <el-radio :value="4096">4096</el-radio>
            </el-radio-group>
            <el-button size="small" type="primary" @click="generateRsaKey" style="margin-left: 12px">
              {{ t('tools.crypto.generateKeys') }}
            </el-button>
          </el-form-item>
          <el-form-item :label="t('tools.crypto.publicKey')">
            <el-input v-model="rsaPublic" type="textarea" :rows="4" readonly />
          </el-form-item>
          <el-form-item :label="t('tools.crypto.privateKey')">
            <el-input v-model="rsaPrivate" type="textarea" :rows="4" readonly />
          </el-form-item>
          <el-form-item :label="t('tools.crypto.plaintext')">
            <el-input v-model="rsaPlain" type="textarea" :rows="3" />
          </el-form-item>
          <el-form-item :label="t('tools.crypto.ciphertext')">
            <el-input v-model="rsaCipher" type="textarea" :rows="3" readonly />
            <el-button size="small" text :icon="DocumentCopy" @click="copy(rsaCipher)">{{ t('common.copy') }}</el-button>
          </el-form-item>
          <el-form-item>
            <el-space>
              <el-button type="primary" :icon="Lock" @click="rsaEncrypt">{{ t('tools.crypto.encryptWithPublic') }}</el-button>
              <el-button :icon="Unlock" @click="rsaDecrypt">{{ t('tools.crypto.decryptWithPrivate') }}</el-button>
            </el-space>
          </el-form-item>
        </el-form>
        <el-alert :title="t('tools.crypto.rsaHelp')" type="info" :closable="false" style="margin-top: 8px" />
      </el-tab-pane>

      <!-- ============ RC4 (legacy stream cipher) ============ -->
      <el-tab-pane :label="t('tools.crypto.rc4')" name="rc4">
        <el-row :gutter="12">
          <el-col :span="12">
            <el-form label-width="120px">
              <el-form-item :label="t('tools.crypto.key')">
                <el-input v-model="rc4Key" />
              </el-form-item>
              <el-form-item>
                <el-space>
                  <el-button type="primary" :icon="Lock" @click="rc4Run(true)">{{ t('tools.crypto.encrypt') }}</el-button>
                  <el-button :icon="Unlock" @click="rc4Run(false)">{{ t('tools.crypto.decrypt') }}</el-button>
                </el-space>
                <div style="margin-top: 4px">
                  <small style="color: var(--text-secondary)">{{ t('tools.crypto.rc4Note') }}</small>
                </div>
              </el-form-item>
            </el-form>
          </el-col>
          <el-col :span="12">
            <el-form label-width="120px">
              <el-form-item :label="t('tools.crypto.plaintext')">
                <el-input v-model="rc4Plain" type="textarea" :rows="5" />
              </el-form-item>
              <el-form-item :label="t('tools.crypto.ciphertext')">
                <el-input v-model="rc4Cipher" type="textarea" :rows="5" readonly />
              </el-form-item>
            </el-form>
          </el-col>
        </el-row>
      </el-tab-pane>

      <!-- ============ Caesar ============ -->
      <el-tab-pane :label="t('tools.crypto.caesar')" name="caesar">
        <el-row :gutter="12">
          <el-col :span="12">
            <el-form label-width="120px">
              <el-form-item :label="t('tools.crypto.shift')">
                <el-input-number v-model="caesarShift" :min="-25" :max="25" />
              </el-form-item>
              <el-form-item>
                <el-space>
                  <el-button type="primary" :icon="Lock" @click="caesarRun(true)">{{ t('tools.crypto.encrypt') }}</el-button>
                  <el-button :icon="Unlock" @click="caesarRun(false)">{{ t('tools.crypto.decrypt') }}</el-button>
                </el-space>
              </el-form-item>
            </el-form>
          </el-col>
          <el-col :span="12">
            <el-form label-width="120px">
              <el-form-item :label="t('tools.crypto.plaintext')">
                <el-input v-model="caesarPlain" type="textarea" :rows="5" />
              </el-form-item>
              <el-form-item :label="t('tools.crypto.ciphertext')">
                <el-input v-model="caesarCipher" type="textarea" :rows="5" readonly />
              </el-form-item>
            </el-form>
          </el-col>
        </el-row>
      </el-tab-pane>

      <!-- ============ Vigenère ============ -->
      <el-tab-pane :label="t('tools.crypto.vigenere')" name="vigenere">
        <el-row :gutter="12">
          <el-col :span="12">
            <el-form label-width="120px">
              <el-form-item :label="t('tools.crypto.key')">
                <el-input v-model="vigKey" :placeholder="t('tools.crypto.vigenereKeyPlaceholder')" />
              </el-form-item>
              <el-form-item>
                <el-space>
                  <el-button type="primary" :icon="Lock" @click="vigenereRun(true)">{{ t('tools.crypto.encrypt') }}</el-button>
                  <el-button :icon="Unlock" @click="vigenereRun(false)">{{ t('tools.crypto.decrypt') }}</el-button>
                </el-space>
              </el-form-item>
            </el-form>
          </el-col>
          <el-col :span="12">
            <el-form label-width="120px">
              <el-form-item :label="t('tools.crypto.plaintext')">
                <el-input v-model="vigPlain" type="textarea" :rows="5" />
              </el-form-item>
              <el-form-item :label="t('tools.crypto.ciphertext')">
                <el-input v-model="vigCipher" type="textarea" :rows="5" readonly />
              </el-form-item>
            </el-form>
          </el-col>
        </el-row>
      </el-tab-pane>

      <!-- ============ XOR ============ -->
      <el-tab-pane :label="t('tools.crypto.xor')" name="xor">
        <el-row :gutter="12">
          <el-col :span="12">
            <el-form label-width="120px">
              <el-form-item :label="t('tools.crypto.key')">
                <el-input v-model="xorKey" :placeholder="t('tools.crypto.xorKeyPlaceholder')" />
                <small style="color: var(--text-secondary)">{{ t('tools.crypto.xorKeyHelp') }}</small>
              </el-form-item>
              <el-form-item>
                <el-space>
                  <el-button type="primary" :icon="Lock" @click="xorRun(true)">{{ t('tools.crypto.encrypt') }}</el-button>
                  <el-button :icon="Unlock" @click="xorRun(false)">{{ t('tools.crypto.decrypt') }}</el-button>
                </el-space>
              </el-form-item>
            </el-form>
          </el-col>
          <el-col :span="12">
            <el-form label-width="120px">
              <el-form-item :label="t('tools.crypto.plaintext')">
                <el-input v-model="xorPlain" type="textarea" :rows="5" />
              </el-form-item>
              <el-form-item :label="t('tools.crypto.ciphertext')">
                <el-input v-model="xorCipher" type="textarea" :rows="5" readonly />
              </el-form-item>
            </el-form>
          </el-col>
        </el-row>
      </el-tab-pane>
    </el-tabs>

    <el-alert v-if="error" type="error" :title="error" :closable="false" show-icon style="margin-top: 12px" />
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { useI18n } from 'vue-i18n'
import { ElMessage } from 'element-plus'
import { Lock, Unlock, DocumentCopy } from '@element-plus/icons-vue'
import { useToolRecorder } from '@/composables/useToolRecorder'

const { t } = useI18n()

const algo = ref<'aes-gcm' | 'aes-cbc' | 'rsa' | 'rc4' | 'caesar' | 'vigenere' | 'xor'>('aes-gcm')
const error = ref('')

// ============ AES-GCM ============
const aesKey = ref('')
const aesIv = ref('')
const aesAad = ref('')
const aesPlain = ref('Hello, Admin Suite!')
const aesCipher = ref('')

function randomAesKey() {
  aesKey.value = toB64(crypto.getRandomValues(new Uint8Array(32)))
}
function randomGcmIv() {
  // 12 bytes is the standard IV size for GCM.
  aesIv.value = toB64(crypto.getRandomValues(new Uint8Array(12)))
}

async function aesGcmEncrypt() {
  error.value = ''
  try {
    const keyBytes = fromB64(aesKey.value)
    if (keyBytes.length !== 16 && keyBytes.length !== 24 && keyBytes.length !== 32) {
      throw new Error(t('tools.crypto.aesKeyLengthError'))
    }
    const ivBytes = fromB64(aesIv.value)
    if (ivBytes.length !== 12) {
      throw new Error(t('tools.crypto.gcmIvLengthError'))
    }
    const cryptoKey = await crypto.subtle.importKey(
      'raw', keyBytes as BufferSource, { name: 'AES-GCM' }, false, ['encrypt']
    )
    const data = new TextEncoder().encode(aesPlain.value)
    const aad = aesAad.value ? new TextEncoder().encode(aesAad.value) : undefined
    const ct = await crypto.subtle.encrypt(
      { name: 'AES-GCM', iv: ivBytes as BufferSource, additionalData: aad as BufferSource },
      cryptoKey,
      data as BufferSource
    )
    // Prepend IV so decrypt knows it.
    const out = new Uint8Array(ivBytes.length + ct.byteLength)
    out.set(ivBytes, 0)
    out.set(new Uint8Array(ct), ivBytes.length)
    aesCipher.value = toB64(out)
  } catch (e: any) {
    error.value = e.message
  }
}

async function aesGcmDecrypt() {
  error.value = ''
  try {
    const all = fromB64(aesCipher.value)
    const ivBytes = all.slice(0, 12)
    const ctBytes = all.slice(12)
    const keyBytes = fromB64(aesKey.value)
    const cryptoKey = await crypto.subtle.importKey(
      'raw', keyBytes as BufferSource, { name: 'AES-GCM' }, false, ['decrypt']
    )
    const aad = aesAad.value ? new TextEncoder().encode(aesAad.value) : undefined
    const pt = await crypto.subtle.decrypt(
      { name: 'AES-GCM', iv: ivBytes as BufferSource, additionalData: aad as BufferSource },
      cryptoKey,
      ctBytes as BufferSource
    )
    aesPlain.value = new TextDecoder().decode(pt)
  } catch (e: any) {
    error.value = e.message
  }
}

// ============ AES-CBC ============
const cbcKey = ref('')
const cbcIv = ref('')
const cbcPlain = ref('Hello, Admin Suite!')
const cbcCipher = ref('')

function randomCbcKey() {
  cbcKey.value = toB64(crypto.getRandomValues(new Uint8Array(32)))
}
function randomCbcIv() {
  // 16 bytes (one AES block).
  cbcIv.value = toB64(crypto.getRandomValues(new Uint8Array(16)))
}

async function aesCbcEncrypt() {
  error.value = ''
  try {
    const keyBytes = fromB64(cbcKey.value)
    if (keyBytes.length !== 16 && keyBytes.length !== 24 && keyBytes.length !== 32) {
      throw new Error(t('tools.crypto.aesKeyLengthError'))
    }
    const ivBytes = fromB64(cbcIv.value)
    if (ivBytes.length !== 16) {
      throw new Error(t('tools.crypto.cbcIvLengthError'))
    }
    const cryptoKey = await crypto.subtle.importKey(
      'raw', keyBytes as BufferSource, { name: 'AES-CBC' }, false, ['encrypt']
    )
    const data = new TextEncoder().encode(cbcPlain.value)
    const ct = await crypto.subtle.encrypt(
      { name: 'AES-CBC', iv: ivBytes as BufferSource },
      cryptoKey,
      data as BufferSource
    )
    const out = new Uint8Array(ivBytes.length + ct.byteLength)
    out.set(ivBytes, 0)
    out.set(new Uint8Array(ct), ivBytes.length)
    cbcCipher.value = toB64(out)
  } catch (e: any) {
    error.value = e.message
  }
}

async function aesCbcDecrypt() {
  error.value = ''
  try {
    const all = fromB64(cbcCipher.value)
    const ivBytes = all.slice(0, 16)
    const ctBytes = all.slice(16)
    const keyBytes = fromB64(cbcKey.value)
    const cryptoKey = await crypto.subtle.importKey(
      'raw', keyBytes as BufferSource, { name: 'AES-CBC' }, false, ['decrypt']
    )
    const pt = await crypto.subtle.decrypt(
      { name: 'AES-CBC', iv: ivBytes as BufferSource },
      cryptoKey,
      ctBytes as BufferSource
    )
    cbcPlain.value = new TextDecoder().decode(pt)
  } catch (e: any) {
    error.value = e.message
  }
}

// ============ RSA-OAEP ============
const rsaSize = ref(2048)
const rsaPublic = ref('')
const rsaPrivate = ref('')
const rsaPlain = ref('Top secret!')
const rsaCipher = ref('')
let rsaKeyPair: CryptoKeyPair | null = null

async function generateRsaKey() {
  error.value = ''
  try {
    rsaKeyPair = await crypto.subtle.generateKey(
      {
        name: 'RSA-OAEP',
        modulusLength: rsaSize.value,
        publicExponent: new Uint8Array([1, 0, 1]),
        hash: 'SHA-256'
      },
      true,
      ['encrypt', 'decrypt']
    )
    const pub = await crypto.subtle.exportKey('spki', rsaKeyPair.publicKey)
    const priv = await crypto.subtle.exportKey('pkcs8', rsaKeyPair.privateKey)
    rsaPublic.value = toPem(pub, 'PUBLIC KEY')
    rsaPrivate.value = toPem(priv, 'PRIVATE KEY')
    ElMessage.success(t('tools.crypto.rsaKeyGenerated'))
  } catch (e: any) {
    error.value = e.message
  }
}

async function rsaEncrypt() {
  error.value = ''
  try {
    if (!rsaKeyPair) throw new Error(t('tools.crypto.rsaKeyMissing'))
    const data = new TextEncoder().encode(rsaPlain.value)
    const ct = await crypto.subtle.encrypt(
      { name: 'RSA-OAEP' },
      rsaKeyPair.publicKey,
      data as BufferSource
    )
    rsaCipher.value = toB64(new Uint8Array(ct))
  } catch (e: any) {
    error.value = e.message
  }
}

async function rsaDecrypt() {
  error.value = ''
  try {
    if (!rsaKeyPair) throw new Error(t('tools.crypto.rsaKeyMissing'))
    const ct = fromB64(rsaCipher.value)
    const pt = await crypto.subtle.decrypt(
      { name: 'RSA-OAEP' },
      rsaKeyPair.privateKey,
      ct as BufferSource
    )
    rsaPlain.value = new TextDecoder().decode(pt)
  } catch (e: any) {
    error.value = e.message
  }
}

// ============ RC4 ============
const rc4Key = ref('secret')
const rc4Plain = ref('Hello, RC4!')
const rc4Cipher = ref('')

function rc4Run(encrypt: boolean) {
  error.value = ''
  try {
    // RC4 is symmetric — encrypt and decrypt are the same operation.
    if (encrypt) {
      const bytes = new TextEncoder().encode(rc4Plain.value)
      rc4Cipher.value = toHex(rc4Bytes(rc4Key.value, bytes))
    } else {
      const bytes = fromHex(rc4Cipher.value)
      const out = rc4Bytes(rc4Key.value, bytes)
      rc4Plain.value = new TextDecoder().decode(out)
    }
  } catch (e: any) {
    error.value = e.message
  }
}

function rc4Bytes(keyStr: string, data: Uint8Array): Uint8Array {
  const key = new TextEncoder().encode(keyStr)
  // KSA
  const S = new Uint8Array(256)
  for (let i = 0; i < 256; i++) S[i] = i
  let j = 0
  for (let i = 0; i < 256; i++) {
    j = (j + S[i] + key[i % key.length]) & 0xff
    ;[S[i], S[j]] = [S[j], S[i]]
  }
  // PRGA
  const out = new Uint8Array(data.length)
  let i = 0
  j = 0
  for (let k = 0; k < data.length; k++) {
    i = (i + 1) & 0xff
    j = (j + S[i]) & 0xff
    ;[S[i], S[j]] = [S[j], S[i]]
    out[k] = S[(S[i] + S[j]) & 0xff] ^ data[k]
  }
  return out
}

// ============ Caesar ============
const caesarShift = ref(3)
const caesarPlain = ref('Hello, World!')
const caesarCipher = ref('')

function caesarRun(encrypt: boolean) {
  error.value = ''
  const shift = ((encrypt ? caesarShift.value : -caesarShift.value) % 26 + 26) % 26
  const s = encrypt ? caesarPlain.value : caesarCipher.value
  let out = ''
  for (const c of s) {
    const code = c.charCodeAt(0)
    if (code >= 65 && code <= 90) {
      out += String.fromCharCode(((code - 65 + shift) % 26) + 65)
    } else if (code >= 97 && code <= 122) {
      out += String.fromCharCode(((code - 97 + shift) % 26) + 97)
    } else {
      out += c
    }
  }
  if (encrypt) caesarCipher.value = out
  else caesarPlain.value = out
}

// ============ Vigenère ============
const vigKey = ref('LEMON')
const vigPlain = ref('ATTACKATDAWN')
const vigCipher = ref('')

function vigenereRun(encrypt: boolean) {
  error.value = ''
  const key = vigKey.value.toUpperCase().replace(/[^A-Z]/g, '')
  if (!key) {
    error.value = t('tools.crypto.vigenereKeyEmpty')
    return
  }
  const src = encrypt ? vigPlain.value : vigCipher.value
  let out = ''
  let ki = 0
  for (const c of src) {
    const code = c.charCodeAt(0)
    if (code >= 65 && code <= 90) {
      const k = key.charCodeAt(ki % key.length) - 65
      const shift = encrypt ? k : -k
      out += String.fromCharCode(((code - 65 + shift + 26 * 100) % 26) + 65)
      ki++
    } else if (code >= 97 && code <= 122) {
      const k = key.charCodeAt(ki % key.length) - 65
      const shift = encrypt ? k : -k
      out += String.fromCharCode(((code - 97 + shift + 26 * 100) % 26) + 97)
      ki++
    } else {
      out += c
    }
  }
  if (encrypt) vigCipher.value = out
  else vigPlain.value = out
}

// ============ XOR ============
const xorKey = ref('deadbeef')
const xorPlain = ref('Hello, XOR!')
const xorCipher = ref('')

function xorRun(_encrypt: boolean) {
  error.value = ''
  // Try to interpret the input as hex; if that fails, treat as text.
  let inBytes: Uint8Array
  try {
    inBytes = fromHex(xorPlain.value)
  } catch {
    inBytes = new TextEncoder().encode(xorPlain.value)
  }
  // Key as bytes — also try hex first.
  let keyBytes: Uint8Array
  try {
    keyBytes = fromHex(xorKey.value)
  } catch {
    keyBytes = new TextEncoder().encode(xorKey.value)
  }
  if (keyBytes.length === 0) {
    error.value = t('tools.crypto.xorKeyEmpty')
    return
  }
  const out = new Uint8Array(inBytes.length)
  for (let i = 0; i < inBytes.length; i++) {
    out[i] = inBytes[i] ^ keyBytes[i % keyBytes.length]
  }
  // If input was plain text, output as hex.  If input was hex, output as text.
  let inputWasText = true
  try {
    fromHex(xorPlain.value)
    inputWasText = false
  } catch { /* keep as text */ }
  if (inputWasText) {
    xorCipher.value = toHex(out)
  } else {
    xorCipher.value = new TextDecoder().decode(out)
  }
}

// ============ Helpers ============
function toB64(bytes: Uint8Array): string {
  let bin = ''
  for (const b of bytes) bin += String.fromCharCode(b)
  return btoa(bin)
}
function fromB64(s: string): Uint8Array {
  const bin = atob(s.trim())
  const out = new Uint8Array(bin.length)
  for (let i = 0; i < bin.length; i++) out[i] = bin.charCodeAt(i)
  return out
}
function toHex(bytes: Uint8Array): string {
  return Array.from(bytes, (b) => b.toString(16).padStart(2, '0')).join('')
}
function fromHex(s: string): Uint8Array {
  const clean = s.replace(/[^0-9a-fA-F]/g, '')
  if (clean.length % 2 !== 0) throw new Error('odd-length hex')
  const out = new Uint8Array(clean.length / 2)
  for (let i = 0; i < out.length; i++) {
    out[i] = parseInt(clean.substr(i * 2, 2), 16)
  }
  return out
}
function toPem(buf: ArrayBuffer, label: string): string {
  const b64 = toB64(new Uint8Array(buf))
  const lines = b64.match(/.{1,64}/g) || []
  return `-----BEGIN ${label}-----\n${lines.join('\n')}\n-----END ${label}-----`
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

// Init with random keys on mount.
randomAesKey()
randomGcmIv()
randomCbcKey()
randomCbcIv()

// Record sanitised snapshots.  Cryptographic tools deliberately do NOT
// persist keys / IVs / private key material — only the algorithm + a
// truncated plaintext head, which is enough to see "what I did last time"
// without ever leaking a secret.
useToolRecorder('/tools/crypto', () => ({
  algo: algo.value,
  aesPlainHead: aesPlain.value.slice(0, 80),
  cbcPlainHead: cbcPlain.value.slice(0, 80),
  rsaPlainHead: rsaPlain.value.slice(0, 80),
  caesarShift: caesarShift.value
}))

onMounted(() => {
  window.addEventListener('admin-suite:restore-snapshot', onRestore as EventListener)
})
function onRestore(ev: Event) {
  const detail = (ev as CustomEvent<{ inputs: Record<string, unknown> }>).detail
  const inputs = detail?.inputs || {}
  if (typeof inputs.algo === 'string') algo.value = inputs.algo as typeof algo.value
  if (typeof inputs.aesPlainHead === 'string') aesPlain.value = inputs.aesPlainHead
  if (typeof inputs.cbcPlainHead === 'string') cbcPlain.value = inputs.cbcPlainHead
  if (typeof inputs.rsaPlainHead === 'string') rsaPlain.value = inputs.rsaPlainHead
  if (typeof inputs.caesarShift === 'number') caesarShift.value = inputs.caesarShift
  ElMessage.success(t('recents.title'))
}
</script>