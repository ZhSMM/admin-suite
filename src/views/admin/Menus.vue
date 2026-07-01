<template>
  <div class="page-container">
    <div class="page-header">
      <h2>{{ t('menus.title') }}</h2>
      <el-button type="primary" @click="openCreate()">
        <el-icon><Plus /></el-icon>
        {{ t('menus.create') }}
      </el-button>
    </div>

    <el-table
      :data="tree"
      v-loading="loading"
      row-key="id"
      :tree-props="{ children: 'children' }"
      :default-expand-all="true"
      border
    >
      <el-table-column :label="t('common.code')" prop="code" width="220" />
      <el-table-column :label="t('common.name')" prop="title" width="160" />
      <el-table-column :label="t('menus.columns.path')" prop="path" width="200" />
      <el-table-column :label="t('menus.columns.icon')" prop="icon" width="120" />
      <el-table-column :label="t('menus.columns.permission')" prop="permission_code" width="160" />
      <el-table-column :label="t('menus.columns.visible')" width="80">
        <template #default="{ row }">
          <el-tag v-if="row.visible" type="success" size="small">yes</el-tag>
          <el-tag v-else type="info" size="small">no</el-tag>
        </template>
      </el-table-column>
      <el-table-column :label="t('common.actions')" width="200" fixed="right">
        <template #default="{ row }">
          <el-button text type="primary" @click="openEdit(row)">{{ t('common.edit') }}</el-button>
          <el-button text type="primary" @click="openCreate(row.id)">
            + child
          </el-button>
          <el-button text type="danger" @click="onDelete(row)">{{ t('common.delete') }}</el-button>
        </template>
      </el-table-column>
    </el-table>

    <el-dialog
      v-model="dialog.open"
      :title="dialog.id ? t('common.edit') : t('menus.create')"
      width="520"
      @closed="resetForm"
    >
      <el-form ref="formRef" :model="form" label-width="120px">
        <el-form-item :label="t('common.code')">
          <el-input v-model="form.code" :disabled="!!dialog.id" />
        </el-form-item>
        <el-form-item :label="t('common.name')">
          <el-input v-model="form.title" />
        </el-form-item>
        <el-form-item :label="t('menus.columns.path')">
          <el-input v-model="form.path" />
        </el-form-item>
        <el-form-item :label="t('menus.columns.icon')">
          <el-input v-model="form.icon" placeholder="user-filled" />
        </el-form-item>
        <el-form-item :label="t('menus.columns.permission')">
          <el-input v-model="form.permission_code" placeholder="user:read" />
        </el-form-item>
        <el-form-item :label="t('common.sort')">
          <el-input-number v-model="form.sort_order" :min="0" />
        </el-form-item>
        <el-form-item :label="t('common.status')">
          <el-select v-model="form.status">
            <el-option label="active" value="active" />
            <el-option label="disabled" value="disabled" />
          </el-select>
        </el-form-item>
        <el-form-item :label="t('menus.columns.visible')">
          <el-switch v-model="form.visible" />
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
import { ElMessage, ElMessageBox } from 'element-plus'
import { useI18n } from 'vue-i18n'
import { useAuthStore } from '@/stores/auth'
import { menusApi, type MenuNode, type MenuCreate, type MenuUpdate, type Menu } from '@/api/roles'

const { t } = useI18n()
const auth = useAuthStore()

const tree = ref<MenuNode[]>([])
const loading = ref(false)
const saving = ref(false)

async function reload() {
  loading.value = true
  try {
    tree.value = await menusApi.tree(auth.token)
  } finally {
    loading.value = false
  }
}
onMounted(reload)

const dialog = reactive<{ open: boolean; id: string | null; parentId: string | null }>({
  open: false,
  id: null,
  parentId: null
})
const form = reactive<MenuCreate>({
  parent_id: null,
  code: '',
  title: '',
  path: '',
  icon: '',
  component: '',
  sort_order: 0,
  visible: true,
  status: 'active',
  menu_type: 'menu',
  permission_code: ''
})

function openCreate(parentId?: string) {
  dialog.id = null
  dialog.parentId = parentId || null
  form.parent_id = parentId || null
  form.code = ''
  form.title = ''
  form.path = ''
  form.icon = ''
  form.sort_order = 0
  form.visible = true
  form.status = 'active'
  form.permission_code = ''
  dialog.open = true
}
function openEdit(row: Menu) {
  dialog.id = row.id
  form.code = row.code
  form.title = row.title
  form.path = row.path || ''
  form.icon = row.icon || ''
  form.sort_order = row.sort_order
  form.visible = row.visible
  form.status = row.status
  form.permission_code = row.permission_code || ''
  form.parent_id = row.parent_id
  dialog.open = true
}
function resetForm() {
  dialog.id = null
  dialog.parentId = null
}

async function onSave() {
  saving.value = true
  try {
    if (dialog.id) {
      const payload: MenuUpdate = { id: dialog.id, title: form.title, path: form.path || null, icon: form.icon || null, sort_order: form.sort_order, visible: form.visible, status: form.status, permission_code: form.permission_code || null, parent_id: form.parent_id }
      await menusApi.update(auth.token, payload)
    } else {
      const payload: MenuCreate = { ...form, path: form.path || null, icon: form.icon || null, permission_code: form.permission_code || null }
      await menusApi.create(auth.token, payload)
    }
    ElMessage.success(t('common.success'))
    dialog.open = false
    await reload()
  } finally {
    saving.value = false
  }
}

async function onDelete(row: Menu) {
  await ElMessageBox.confirm(t('common.confirmDelete'), '', { type: 'warning' })
  await menusApi.remove(auth.token, row.id)
  ElMessage.success(t('common.success'))
  await reload()
}
</script>