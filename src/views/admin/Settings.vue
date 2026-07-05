<template>
  <div class="page-container">
    <div class="page-header">
      <h2>{{ t('settings.title') }}</h2>
      <el-button type="primary" :loading="saving" :icon="Check" @click="save">
        {{ t('common.save') }}
      </el-button>
    </div>

    <el-alert :title="t('settings.help')" type="info" :closable="false" style="margin-bottom: 16px" />

    <!-- Session -->
    <el-card shadow="never" style="margin-bottom: 16px">
      <template #header>
        <strong>{{ t('settings.section.session') }}</strong>
      </template>
      <el-form label-width="240px">
        <el-form-item :label="t('settings.session.timeoutMinutes')">
          <el-input-number v-model="form.session_timeout_minutes" :min="5" :max="1440" :step="15" />
          <small style="margin-left: 12px; color: var(--text-secondary)">
            {{ t('settings.session.timeoutHelp') }}
          </small>
        </el-form-item>
      </el-form>
    </el-card>

    <!-- Auth -->
    <el-card shadow="never" style="margin-bottom: 16px">
      <template #header>
        <strong>{{ t('settings.section.auth') }}</strong>
      </template>
      <el-form label-width="240px">
        <el-form-item :label="t('settings.auth.passwordMinLength')">
          <el-input-number v-model="form.auth_password_min_length" :min="4" :max="128" />
        </el-form-item>
        <el-form-item :label="t('settings.auth.loginMaxFailures')">
          <el-input-number v-model="form.auth_login_max_failures" :min="1" :max="1000" />
        </el-form-item>
        <el-form-item :label="t('settings.auth.lockoutMinutes')">
          <el-input-number v-model="form.auth_lockout_minutes" :min="1" :max="1440" />
          <small style="margin-left: 12px; color: var(--text-secondary)">
            {{ t('settings.auth.lockoutHelp') }}
          </small>
        </el-form-item>
      </el-form>
    </el-card>

    <!-- Backup -->
    <el-card shadow="never" style="margin-bottom: 16px">
      <template #header>
        <strong>{{ t('settings.section.backup') }}</strong>
      </template>
      <el-form label-width="240px">
        <el-form-item :label="t('settings.backup.autoOnStart')">
          <el-switch v-model="form.backup_auto_on_start" />
        </el-form-item>
        <el-form-item :label="t('settings.backup.keepCount')">
          <el-input-number v-model="form.backup_keep_count" :min="1" :max="100" />
          <small style="margin-left: 12px; color: var(--text-secondary)">
            {{ t('settings.backup.keepCountHelp') }}
          </small>
        </el-form-item>
      </el-form>
    </el-card>

    <!-- UI -->
    <el-card shadow="never">
      <template #header>
        <strong>{{ t('settings.section.ui') }}</strong>
      </template>
      <el-form label-width="240px">
        <el-form-item :label="t('settings.ui.commandPalette')">
          <el-switch v-model="form.ui_command_palette" />
          <small style="margin-left: 12px; color: var(--text-secondary)">
            {{ t('settings.ui.commandPaletteHelp') }}
          </small>
        </el-form-item>
      </el-form>
    </el-card>

    <!-- AI -->
    <el-card shadow="never" style="margin-top: 16px">
      <template #header>
        <strong>{{ t('settings.section.ai') }}</strong>
      </template>
      <el-alert :title="t('settings.ai.help')" type="info" :closable="false" style="margin-bottom: 12px" />

      <!-- v0.6.2 — One-click local install -->
      <LocalModelPanel />
      <el-form label-width="240px">
        <el-form-item :label="t('settings.ai.defaultChat')">
          <el-select
            v-model="form.ai_default_chat_provider"
            :placeholder="t('settings.ai.pickProvider')"
            style="width: 200px"
            clearable
          >
            <el-option
              v-for="p in llm.enabledProviders"
              :key="p.id"
              :label="p.name"
              :value="p.id"
            />
          </el-select>
          <el-select
            v-model="form.ai_default_chat_model"
            :placeholder="t('settings.ai.pickModel')"
            style="width: 240px; margin-left: 8px"
            :disabled="!form.ai_default_chat_provider"
            clearable
          >
            <el-option
              v-for="m in chatModelsFor(form.ai_default_chat_provider)"
              :key="m.id"
              :label="m.display_name"
              :value="m.id"
            />
          </el-select>
        </el-form-item>
        <el-form-item :label="t('settings.ai.defaultTranslate')">
          <el-select
            v-model="form.ai_default_translate_provider"
            :placeholder="t('settings.ai.pickProvider')"
            style="width: 200px"
            clearable
          >
            <el-option
              v-for="p in llm.enabledProviders"
              :key="p.id"
              :label="p.name"
              :value="p.id"
            />
          </el-select>
          <el-select
            v-model="form.ai_default_translate_model"
            :placeholder="t('settings.ai.pickModel')"
            style="width: 240px; margin-left: 8px"
            :disabled="!form.ai_default_translate_provider"
            clearable
          >
            <el-option
              v-for="m in chatModelsFor(form.ai_default_translate_provider)"
              :key="m.id"
              :label="m.display_name"
              :value="m.id"
            />
          </el-select>
        </el-form-item>
        <el-form-item :label="t('settings.ai.localFirst')">
          <el-switch v-model="form.ai_local_first" />
          <small style="margin-left: 12px; color: var(--text-secondary)">
            {{ t('settings.ai.localFirstHelp') }}
          </small>
        </el-form-item>
      </el-form>
    </el-card>
  </div>
</template>

<script setup lang="ts">
import { onMounted, reactive, ref } from 'vue'
import { ElMessage } from 'element-plus'
import { Check } from '@element-plus/icons-vue'
import { useI18n } from 'vue-i18n'
import { useAuthStore } from '@/stores/auth'
import { useLlmStore } from '@/stores/llm'
import { settingsApi, type Setting, type SettingUpdate } from '@/api/settings'
import LocalModelPanel from '@/views/ai/LocalModelPanel.vue'

const { t } = useI18n()
const auth = useAuthStore()
const llm = useLlmStore()

const saving = ref(false)

// Form keys are stored in this shape so the backend's snake_case / dotted
// names stay out of the template.  Each property maps to one app_state row.
const form = reactive({
  session_timeout_minutes: 480,
  auth_password_min_length: 6,
  auth_login_max_failures: 10,
  auth_lockout_minutes: 15,
  backup_auto_on_start: true,
  backup_keep_count: 10,
  ui_command_palette: true,
  ai_default_chat_provider: '',
  ai_default_chat_model: '',
  ai_default_translate_provider: '',
  ai_default_translate_model: '',
  ai_local_first: false
})

function chatModelsFor(providerId: string) {
  return providerId ? llm.modelsFor(providerId) : []
}

function apply(rows: Setting[]) {
  for (const r of rows) {
    const v = r.value
    switch (r.key) {
      case 'session.timeout_minutes': form.session_timeout_minutes = parseInt(v, 10) || 480; break
      case 'auth.password_min_length': form.auth_password_min_length = parseInt(v, 10) || 6; break
      case 'auth.login_max_failures': form.auth_login_max_failures = parseInt(v, 10) || 10; break
      case 'auth.lockout_minutes': form.auth_lockout_minutes = parseInt(v, 10) || 15; break
      case 'backup.auto_on_start': form.backup_auto_on_start = v === 'true'; break
      case 'backup.keep_count': form.backup_keep_count = parseInt(v, 10) || 10; break
      case 'ui.command_palette': form.ui_command_palette = v === 'true'; break
      case 'ai.default_chat_provider': form.ai_default_chat_provider = v; break
      case 'ai.default_chat_model': form.ai_default_chat_model = v; break
      case 'ai.default_translate_provider': form.ai_default_translate_provider = v; break
      case 'ai.default_translate_model': form.ai_default_translate_model = v; break
      case 'ai.local_first': form.ai_local_first = v === 'true'; break
    }
  }
}

async function reload() {
  try {
    const rows = await settingsApi.list(auth.token)
    apply(rows)
    // Ensure providers are loaded so the dropdowns render.
    if (llm.providers.length === 0) {
      await llm.loadAll(auth.token || '')
    }
  } catch (e) {
    ElMessage.error((e as Error).message)
  }
}

async function save() {
  saving.value = true
  try {
    const updates: SettingUpdate[] = [
      { key: 'session.timeout_minutes', value: String(form.session_timeout_minutes) },
      { key: 'auth.password_min_length', value: String(form.auth_password_min_length) },
      { key: 'auth.login_max_failures', value: String(form.auth_login_max_failures) },
      { key: 'auth.lockout_minutes', value: String(form.auth_lockout_minutes) },
      { key: 'backup.auto_on_start', value: form.backup_auto_on_start ? 'true' : 'false' },
      { key: 'backup.keep_count', value: String(form.backup_keep_count) },
      { key: 'ui.command_palette', value: form.ui_command_palette ? 'true' : 'false' },
      { key: 'ai.default_chat_provider', value: form.ai_default_chat_provider },
      { key: 'ai.default_chat_model', value: form.ai_default_chat_model },
      { key: 'ai.default_translate_provider', value: form.ai_default_translate_provider },
      { key: 'ai.default_translate_model', value: form.ai_default_translate_model },
      { key: 'ai.local_first', value: form.ai_local_first ? 'true' : 'false' }
    ]
    const rows = await settingsApi.set(auth.token, updates)
    apply(rows)
    ElMessage.success(t('common.success'))
  } catch (e) {
    ElMessage.error((e as Error).message)
  } finally {
    saving.value = false
  }
}

onMounted(reload)
</script>