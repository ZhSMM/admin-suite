<template>
  <div class="page-container">
    <div class="page-header">
      <h2>{{ t('roles.title') }}</h2>
      <el-button type="primary" @click="openCreate">
        <el-icon><Plus /></el-icon>
        {{ t('roles.create') }}
      </el-button>
    </div>

    <el-table :data="roles" v-loading="loading" border>
      <el-table-column :label="t('roles.columns.code')" prop="code" width="160" />
      <el-table-column :label="t('roles.columns.name')" prop="name" width="160" />
      <el-table-column :label="t('common.description')" prop="description" />
      <el-table-column :label="t('roles.columns.permissions')">
        <template #default="{ row }">
          <el-tag
            v-for="c in row.permission_codes.slice(0, 6)"
            :key="c"
            size="small"
            style="margin-right: 4px; margin-bottom: 4px"
          >
            {{ c }}
          </el-tag>
          <span v-if="row.permission_codes.length > 6" style="color: var(--text-secondary)">
            +{{ row.permission_codes.length - 6 }}
          </span>
        </template>
      </el-table-column>
      <el-table-column :label="t('roles.columns.builtIn')" width="100">
        <template #default="{ row }">
          <el-tag v-if="row.built_in" type="info" size="small">{{ t('common.builtIn') }}</el-tag>
        </template>
      </el-table-column>
      <el-table-column :label="t('common.actions')" width="280" fixed="right">
        <template #default="{ row }">
          <el-button text type="primary" @click="openEdit(row)">{{ t('common.edit') }}</el-button>
          <el-button text type="primary" @click="openPermissions(row)">
            {{ t('roles.assignPermissions') }}
          </el-button>
          <el-button text type="primary" @click="openMenus(row)">
            {{ t('roles.assignMenus') }}
          </el-button>
          <el-button text type="danger" :disabled="row.built_in" @click="onDelete(row)">
            {{ t('common.delete') }}
          </el-button>
        </template>
      </el-table-column>
    </el-table>

    <!-- Edit dialog -->
    <el-dialog
      v-model="dialog.open"
      :title="dialog.id ? t('common.edit') : t('roles.create')"
      width="520"
      @closed="resetForm"
    >
      <el-form ref="formRef" :model="form" :rules="rules" label-width="100px">
        <el-form-item :label="t('roles.columns.code')" prop="code">
          <el-input v-model="form.code" :disabled="!!dialog.id" />
        </el-form-item>
        <el-form-item :label="t('roles.columns.name')" prop="name">
          <el-input v-model="form.name" />
        </el-form-item>
        <el-form-item :label="t('common.description')">
          <el-input v-model="form.description" type="textarea" :rows="2" />
        </el-form-item>
        <el-form-item :label="t('common.sort')">
          <el-input-number v-model="form.sort_order" :min="0" />
        </el-form-item>
        <el-form-item :label="t('common.status')">
          <el-select v-model="form.status">
            <el-option :label="t('users.statusActive')" value="active" />
            <el-option :label="t('users.statusDisabled')" value="disabled" />
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

    <!-- Permission assignment -->
    <el-dialog v-model="permDialog.open" :title="t('roles.assignPermissions')" width="640">
      <el-transfer
        v-model="permDialog.selected"
        :data="permDialog.options"
        :titles="[t('roles.transfer.available'), t('roles.transfer.selected')]"
        filterable
        :filter-placeholder="t('common.search')"
      />
      <template #footer>
        <el-button @click="permDialog.open = false">{{ t('common.cancel') }}</el-button>
        <el-button type="primary" :loading="saving" @click="savePermissions">
          {{ t('common.save') }}
        </el-button>
      </template>
    </el-dialog>

    <!-- Menu assignment -->
    <el-dialog v-model="menuDialog.open" :title="t('roles.assignMenus')" width="520">
      <el-tree
        ref="menuTreeRef"
        :data="menuTree"
        :props="{ label: 'title', children: 'children' }"
        show-checkbox
        node-key="id"
        default-expand-all
      />
      <template #footer>
        <el-button @click="menuDialog.open = false">{{ t('common.cancel') }}</el-button>
        <el-button type="primary" :loading="saving" @click="saveMenus">
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
import {
  rolesApi,
  menusApi,
  permissionsApi,
  type Role,
  type RoleCreate,
  type RoleUpdate,
  type Permission,
  type MenuNode
} from '@/api/roles'

const { t } = useI18n()
const auth = useAuthStore()

const roles = ref<Role[]>([])
const loading = ref(false)
const saving = ref(false)

async function reload() {
  loading.value = true
  try {
    roles.value = await rolesApi.list(auth.token)
  } finally {
    loading.value = false
  }
}
onMounted(reload)

const dialog = reactive<{ open: boolean; id: string | null }>({ open: false, id: null })
const formRef = ref<FormInstance>()
const form = reactive<{ code: string; name: string; description: string; sort_order: number; status: string }>({
  code: '',
  name: '',
  description: '',
  sort_order: 100,
  status: 'active'
})
const rules: FormRules = {
  code: [{ required: true, message: () => t('roles.validation.codeRequired'), trigger: 'blur' }],
  name: [{ required: true, message: () => t('roles.validation.nameRequired'), trigger: 'blur' }]
}

function openCreate() {
  resetForm()
  dialog.open = true
}
function openEdit(row: Role) {
  resetForm()
  dialog.id = row.id
  form.code = row.code
  form.name = row.name
  form.description = row.description || ''
  form.sort_order = row.sort_order
  form.status = row.status
  dialog.open = true
}
function resetForm() {
  dialog.id = null
  form.code = ''
  form.name = ''
  form.description = ''
  form.sort_order = 100
  form.status = 'active'
}

async function onSave() {
  await formRef.value!.validate()
  saving.value = true
  try {
    if (dialog.id) {
      const payload: RoleUpdate = {
        id: dialog.id,
        name: form.name,
        description: form.description || null,
        sort_order: form.sort_order,
        status: form.status
      }
      await rolesApi.update(auth.token, payload)
    } else {
      const payload: RoleCreate = {
        code: form.code,
        name: form.name,
        description: form.description || null,
        sort_order: form.sort_order,
        status: form.status,
        permission_ids: []
      }
      await rolesApi.create(auth.token, payload)
    }
    ElMessage.success(t('common.success'))
    dialog.open = false
    await reload()
  } finally {
    saving.value = false
  }
}

async function onDelete(row: Role) {
  await ElMessageBox.confirm(t('common.confirmDelete'), '', { type: 'warning' })
  await rolesApi.remove(auth.token, row.id)
  ElMessage.success(t('common.success'))
  await reload()
}

// ---- Permission assignment ----
const permDialog = reactive<{
  open: boolean
  roleId: string | null
  options: { key: string; label: string }[]
  selected: string[]
}>({ open: false, roleId: null, options: [], selected: [] })
async function openPermissions(row: Role) {
  const all: Permission[] = await permissionsApi.list(auth.token)
  permDialog.options = all.map((p) => ({ key: p.id, label: `${p.code} (${p.name})` }))
  permDialog.selected = all
    .filter((p) => row.permission_codes.includes(p.code))
    .map((p) => p.id)
  permDialog.roleId = row.id
  permDialog.open = true
}
async function savePermissions() {
  if (!permDialog.roleId) return
  saving.value = true
  try {
    await rolesApi.update(auth.token, {
      id: permDialog.roleId,
      permission_ids: permDialog.selected
    })
    ElMessage.success(t('common.success'))
    permDialog.open = false
    await reload()
  } finally {
    saving.value = false
  }
}

// ---- Menu assignment ----
const menuDialog = reactive<{ open: boolean; roleId: string | null }>({ open: false, roleId: null })
const menuTree = ref<MenuNode[]>([])
const menuTreeRef = ref<any>()
async function openMenus(row: Role) {
  menuTree.value = await menusApi.tree(auth.token)
  const selected = await rolesApi.getMenus(auth.token, row.id)
  // Tree is rendered; set checked after dialog mounts.
  menuDialog.roleId = row.id
  menuDialog.open = true
  // Vue renders the tree asynchronously; queue the check on the next tick.
  setTimeout(() => {
    if (menuTreeRef.value) {
      menuTreeRef.value.setCheckedKeys(selected, false)
    }
  }, 0)
}
async function saveMenus() {
  if (!menuDialog.roleId || !menuTreeRef.value) return
  const checked = menuTreeRef.value.getCheckedKeys() as string[]
  const halfChecked = menuTreeRef.value.getHalfCheckedKeys() as string[]
  // We persist only fully-checked leaves/parents so the role-menu table stays clean.
  saving.value = true
  try {
    await rolesApi.assignMenus(auth.token, menuDialog.roleId, [...checked, ...halfChecked])
    ElMessage.success(t('common.success'))
    menuDialog.open = false
    await reload()
  } finally {
    saving.value = false
  }
}
</script>