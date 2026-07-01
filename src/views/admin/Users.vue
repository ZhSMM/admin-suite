<template>
  <div class="page-container">
    <div class="page-header">
      <h2>{{ t('users.title') }}</h2>
      <div>
        <el-input
          v-model="query.keyword"
          :placeholder="t('common.search')"
          clearable
          style="width: 200px; margin-right: 8px"
          @change="reload"
        />
        <el-select
          v-model="query.status"
          :placeholder="t('common.status')"
          clearable
          style="width: 140px; margin-right: 8px"
          @change="reload"
        >
          <el-option label="active" value="active" />
          <el-option label="disabled" value="disabled" />
        </el-select>
        <el-button type="primary" @click="openCreate">
          <el-icon><Plus /></el-icon>
          {{ t('users.create') }}
        </el-button>
      </div>
    </div>

    <el-table :data="list.items" v-loading="loading" border style="width: 100%">
      <el-table-column :label="t('users.columns.username')" prop="username" width="140" />
      <el-table-column :label="t('users.columns.displayName')" prop="display_name" width="160" />
      <el-table-column :label="t('users.columns.email')" prop="email" width="200" />
      <el-table-column :label="t('users.columns.roles')" width="180">
        <template #default="{ row }">
          <el-tag
            v-for="c in row.role_codes"
            :key="c"
            size="small"
            style="margin-right: 4px"
          >
            {{ c }}
          </el-tag>
        </template>
      </el-table-column>
      <el-table-column :label="t('users.columns.status')" width="100">
        <template #default="{ row }">
          <el-tag :type="row.status === 'active' ? 'success' : 'info'">{{ row.status }}</el-tag>
        </template>
      </el-table-column>
      <el-table-column :label="t('users.columns.lastLogin')" width="170">
        <template #default="{ row }">{{ row.last_login_at || '-' }}</template>
      </el-table-column>
      <el-table-column :label="t('common.actions')" width="200" fixed="right">
        <template #default="{ row }">
          <el-button text type="primary" @click="openEdit(row)">{{ t('common.edit') }}</el-button>
          <el-button
            text
            type="danger"
            :disabled="row.is_super_admin"
            @click="onDelete(row)"
          >
            {{ t('common.delete') }}
          </el-button>
        </template>
      </el-table-column>
    </el-table>

    <el-pagination
      v-model:current-page="query.page"
      v-model:page-size="query.page_size"
      :total="list.total"
      :page-sizes="[10, 20, 50, 100]"
      layout="total, sizes, prev, pager, next, jumper"
      style="margin-top: 16px; justify-content: flex-end"
      @current-change="reload"
      @size-change="reload"
    />

    <el-dialog
      v-model="dialog.open"
      :title="dialog.id ? t('common.edit') : t('users.create')"
      width="520"
      @closed="resetForm"
    >
      <el-form ref="formRef" :model="form" :rules="rules" label-width="100px">
        <el-form-item :label="t('users.columns.username')" prop="username">
          <el-input v-model="form.username" :disabled="!!dialog.id" />
        </el-form-item>
        <el-form-item :label="t('users.columns.displayName')" prop="display_name">
          <el-input v-model="form.display_name" />
        </el-form-item>
        <el-form-item :label="t('users.password')" :prop="dialog.id ? '' : 'password'">
          <el-input v-model="form.password" type="password" show-password />
          <small style="color: var(--text-secondary)">
            {{ dialog.id ? t('users.passwordHelp') : t('common.optional') }}
          </small>
        </el-form-item>
        <el-form-item :label="t('users.columns.email')">
          <el-input v-model="form.email" />
        </el-form-item>
        <el-form-item :label="t('common.status')">
          <el-select v-model="form.status">
            <el-option label="active" value="active" />
            <el-option label="disabled" value="disabled" />
          </el-select>
        </el-form-item>
        <el-form-item :label="t('users.columns.roles')">
          <el-select v-model="form.role_ids" multiple filterable style="width: 100%">
            <el-option
              v-for="r in roles"
              :key="r.id"
              :label="`${r.code} (${r.name})`"
              :value="r.id"
            />
          </el-select>
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="dialog.open = false">{{ t('common.cancel') }}</el-button>
        <el-button type="primary" :loading="saving" @click="onSave">
          {{ t('common.save') }}
        </el-button>
      </template>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { onMounted, reactive, ref } from 'vue'
import { ElMessage, ElMessageBox, type FormInstance, type FormRules } from 'element-plus'
import { useI18n } from 'vue-i18n'
import { useAuthStore } from '@/stores/auth'
import { usersApi, type UserCreate, type UserListResult, type UserSafe, type UserUpdate } from '@/api/users'
import { rolesApi, type Role } from '@/api/roles'

const { t } = useI18n()
const auth = useAuthStore()

const list = ref<UserListResult>({ items: [], total: 0, page: 1, page_size: 20 })
const roles = ref<Role[]>([])
const loading = ref(false)
const saving = ref(false)
const query = reactive({
  keyword: '',
  status: '',
  role_id: '',
  page: 1,
  page_size: 20
})

async function reload() {
  loading.value = true
  try {
    list.value = await usersApi.list(auth.token, query)
  } finally {
    loading.value = false
  }
}

onMounted(async () => {
  await reload()
  roles.value = await rolesApi.list(auth.token)
})

const dialog = reactive<{ open: boolean; id: string | null }>({ open: false, id: null })
const formRef = ref<FormInstance>()
const form = reactive<UserCreate & { id?: string; password: string }>({
  username: '',
  display_name: '',
  password: '',
  email: '',
  phone: '',
  avatar: '',
  status: 'active',
  role_ids: []
})
const rules: FormRules = {
  username: [{ required: true, message: 'username required', trigger: 'blur' }],
  display_name: [{ required: true, message: 'display name required', trigger: 'blur' }],
  password: [{ required: false, validator: (_r, v, cb) => {
    if (!v || (v as string).length >= 6) cb()
    else cb(new Error('>= 6 chars'))
  }, trigger: 'blur' }]
}

function openCreate() {
  resetForm()
  dialog.open = true
}
function openEdit(row: UserSafe) {
  resetForm()
  dialog.id = row.id
  form.username = row.username
  form.display_name = row.display_name
  form.email = row.email || ''
  form.status = row.status
  form.role_ids = [...row.role_ids]
  dialog.open = true
}
function resetForm() {
  dialog.id = null
  form.username = ''
  form.display_name = ''
  form.password = ''
  form.email = ''
  form.status = 'active'
  form.role_ids = []
}

async function onSave() {
  if (!formRef.value) return
  await formRef.value.validate()
  saving.value = true
  try {
    if (dialog.id) {
      const payload: UserUpdate = {
        id: dialog.id,
        display_name: form.display_name,
        email: form.email || null,
        status: form.status,
        role_ids: form.role_ids
      }
      if (form.password) payload.password = form.password
      await usersApi.update(auth.token, payload)
    } else {
      const payload: UserCreate = {
        username: form.username,
        display_name: form.display_name,
        password: form.password,
        email: form.email || null,
        status: form.status,
        role_ids: form.role_ids
      }
      await usersApi.create(auth.token, payload)
    }
    ElMessage.success(t('common.success'))
    dialog.open = false
    await reload()
  } finally {
    saving.value = false
  }
}

async function onDelete(row: UserSafe) {
  await ElMessageBox.confirm(t('common.confirmDelete'), '', { type: 'warning' })
  await usersApi.remove(auth.token, row.id)
  ElMessage.success(t('common.success'))
  await reload()
}
</script>