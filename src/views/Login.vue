<template>
  <div class="login-bg">
    <el-card class="login-card" shadow="always">
      <template #header>
        <div class="login-header">
          <h2>Admin Suite</h2>
          <span>{{ t('auth.welcomeBack') }}</span>
        </div>
      </template>

      <el-form
        ref="formRef"
        :model="form"
        :rules="rules"
        label-position="top"
        @submit.prevent="onSubmit"
      >
        <el-form-item :label="t('auth.username')" prop="username">
          <el-input v-model="form.username" :prefix-icon="User" autofocus />
        </el-form-item>
        <el-form-item :label="t('auth.password')" prop="password">
          <el-input
            v-model="form.password"
            type="password"
            show-password
            :prefix-icon="Lock"
            @keyup.enter="onSubmit"
          />
        </el-form-item>
        <el-button
          type="primary"
          :loading="auth.loading"
          class="login-btn"
          @click="onSubmit"
        >
          {{ t('auth.login') }}
        </el-button>
        <el-alert
          v-if="errorMsg"
          type="error"
          :title="errorMsg"
          :closable="false"
          show-icon
          class="error"
        />
      </el-form>
    </el-card>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { useI18n } from 'vue-i18n'
import { ElMessage, type FormInstance, type FormRules } from 'element-plus'
import { User, Lock } from '@element-plus/icons-vue'
import { useAuthStore } from '@/stores/auth'
import { useMenuStore } from '@/stores/menu'
import { useThemeStore } from '@/stores/theme'
import { useLocaleStore } from '@/stores/locale'
import { ApiException } from '@/api'

const route = useRoute()
const router = useRouter()
const auth = useAuthStore()
const menu = useMenuStore()
const theme = useThemeStore()
const localeStore = useLocaleStore()
const { t } = useI18n()

const formRef = ref<FormInstance>()
const errorMsg = ref('')
const form = reactive({
  username: 'admin',
  password: 'admin123'
})
const rules: FormRules = {
  username: [{ required: true, message: () => 'username required', trigger: 'blur' }],
  password: [{ required: true, message: () => 'password required', trigger: 'blur' }]
}

async function onSubmit() {
  errorMsg.value = ''
  if (!formRef.value) return
  try {
    await formRef.value.validate()
  } catch {
    return
  }
  try {
    const r = await auth.login(form.username, form.password)
    menu.setFromLogin(r.menus)
    // Hydrate theme + locale stores from the session.
    await theme.hydrate()
    await localeStore.hydrate()
    ElMessage.success(t('auth.loginSuccess'))
    const target = (route.query.redirect as string) || '/dashboard'
    router.replace(target)
  } catch (e) {
    if (e instanceof ApiException && e.code === 'UNAUTHORIZED') {
      errorMsg.value = t('auth.invalidCredentials')
    } else {
      errorMsg.value = (e as Error).message
    }
  }
}
</script>

<style scoped lang="scss">
.login-bg {
  height: 100vh;
  display: flex;
  align-items: center;
  justify-content: center;
  background: linear-gradient(135deg, #4f46e5 0%, #06b6d4 100%);
}
.login-card {
  width: 360px;
  border-radius: 12px;
}
.login-header {
  text-align: center;
  h2 {
    margin: 0;
  }
  span {
    color: var(--text-secondary);
    font-size: 13px;
  }
}
.login-btn {
  width: 100%;
}
.error {
  margin-top: 12px;
}
</style>