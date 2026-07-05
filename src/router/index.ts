import { createRouter, createWebHashHistory, type RouteRecordRaw } from 'vue-router'
import { useAuthStore } from '@/stores/auth'
import { useLocaleStore } from '@/stores/locale'

declare module 'vue-router' {
  interface RouteMeta {
    requiresAuth?: boolean
    permission?: string
    title?: string
    publicRoute?: boolean
  }
}

const routes: RouteRecordRaw[] = [
  {
    path: '/login',
    name: 'login',
    component: () => import('@/views/Login.vue'),
    meta: { publicRoute: true, title: 'Login' }
  },
  {
    path: '/',
    component: () => import('@/layouts/DefaultLayout.vue'),
    meta: { requiresAuth: true },
    children: [
      {
        path: '',
        redirect: '/dashboard'
      },
      {
        path: 'dashboard',
        name: 'dashboard',
        component: () => import('@/views/Dashboard.vue'),
        meta: { requiresAuth: true, title: 'menu.dashboard' }
      },
      {
        path: 'system/users',
        name: 'users',
        component: () => import('@/views/admin/Users.vue'),
        meta: { requiresAuth: true, permission: 'user:read', title: 'menu.users' }
      },
      {
        path: 'system/roles',
        name: 'roles',
        component: () => import('@/views/admin/Roles.vue'),
        meta: { requiresAuth: true, permission: 'role:read', title: 'menu.roles' }
      },
      {
        path: 'system/menus',
        name: 'menus',
        component: () => import('@/views/admin/Menus.vue'),
        meta: { requiresAuth: true, permission: 'menu:read', title: 'menu.menus' }
      },
      {
        path: 'system/permissions',
        name: 'permissions',
        component: () => import('@/views/admin/Permissions.vue'),
        meta: { requiresAuth: true, permission: 'permission:read', title: 'menu.permissions' }
      },
      {
        path: 'system/themes',
        name: 'themes',
        component: () => import('@/views/admin/Themes.vue'),
        meta: { requiresAuth: true, permission: 'theme:manage', title: 'menu.themes' }
      },
      {
        path: 'system/locales',
        name: 'locales',
        component: () => import('@/views/admin/Locales.vue'),
        meta: { requiresAuth: true, permission: 'locale:manage', title: 'menu.locales' }
      },
      {
        path: 'system/audit',
        name: 'audit',
        component: () => import('@/views/admin/Audit.vue'),
        meta: { requiresAuth: true, permission: 'audit:read', title: 'menu.audit' }
      },
      {
        path: 'system/settings',
        name: 'settings',
        component: () => import('@/views/admin/Settings.vue'),
        meta: { requiresAuth: true, permission: 'settings:manage', title: 'menu.settings' }
      },
      {
        path: 'system/backups',
        name: 'backups',
        component: () => import('@/views/admin/Backups.vue'),
        meta: { requiresAuth: true, permission: 'backup:manage', title: 'menu.backups' }
      },
      {
        path: 'system/monitoring',
        name: 'monitoring',
        component: () => import('@/views/admin/Monitoring.vue'),
        meta: { requiresAuth: true, permission: 'monitoring:read', title: 'menu.monitoring' }
      },
      {
        path: 'system/diagnostics',
        name: 'diagnostics',
        component: () => import('@/views/admin/Diagnostics.vue'),
        meta: { requiresAuth: true, permission: 'diagnostics:read', title: 'menu.diagnostics' }
      },
      {
        path: 'system/updater',
        name: 'updater',
        component: () => import('@/views/admin/Updater.vue'),
        meta: { requiresAuth: true, permission: 'updater:check', title: 'menu.updater' }
      },
      {
        path: 'system/llm/providers',
        name: 'llm-providers',
        component: () => import('@/views/admin/LlmProviders.vue'),
        meta: { requiresAuth: true, permission: 'llm:manage', title: 'menu.llm.providers' }
      },
      {
        path: 'system/llm/models',
        name: 'llm-models',
        component: () => import('@/views/admin/LlmModels.vue'),
        meta: { requiresAuth: true, permission: 'llm:manage', title: 'menu.llm.models' }
      },
      {
        path: 'system/llm/usage',
        name: 'llm-usage',
        component: () => import('@/views/admin/LlmUsage.vue'),
        meta: { requiresAuth: true, permission: 'llm:usage:read', title: 'menu.llm.usage' }
      },
      {
        path: 'ai/chat',
        name: 'ai-chat',
        component: () => import('@/views/ai/Chat.vue'),
        meta: { requiresAuth: true, permission: 'llm:use', title: 'menu.ai.chat' }
      },
      {
        path: 'tools/base',
        name: 'tool-base',
        component: () => import('@/views/tools/BaseConvert.vue'),
        meta: { requiresAuth: true, permission: 'tool:use', title: 'tools.base.title' }
      },
      {
        path: 'tools/json',
        name: 'tool-json',
        component: () => import('@/views/tools/JsonFormatter.vue'),
        meta: { requiresAuth: true, permission: 'tool:use', title: 'tools.json.title' }
      },
      {
        path: 'tools/datetime',
        name: 'tool-datetime',
        component: () => import('@/views/tools/DateTime.vue'),
        meta: { requiresAuth: true, permission: 'tool:use', title: 'tools.datetime.title' }
      },
      {
        path: 'tools/sql',
        name: 'tool-sql',
        component: () => import('@/views/tools/SqlFormatter.vue'),
        meta: { requiresAuth: true, permission: 'tool:use', title: 'tools.sql.title' }
      },
      {
        path: 'tools/encode',
        name: 'tool-encode',
        component: () => import('@/views/tools/Encode.vue'),
        meta: { requiresAuth: true, permission: 'tool:use', title: 'tools.encode.title' }
      },
      {
        path: 'tools/hash',
        name: 'tool-hash',
        component: () => import('@/views/tools/Hash.vue'),
        meta: { requiresAuth: true, permission: 'tool:use', title: 'tools.hash.title' }
      },
      {
        path: 'tools/generate',
        name: 'tool-generate',
        component: () => import('@/views/tools/Generate.vue'),
        meta: { requiresAuth: true, permission: 'tool:use', title: 'tools.gen.title' }
      },
      {
        path: 'tools/regex',
        name: 'tool-regex',
        component: () => import('@/views/tools/Regex.vue'),
        meta: { requiresAuth: true, permission: 'tool:use', title: 'tools.regex.title' }
      },
      {
        path: 'tools/diff',
        name: 'tool-diff',
        component: () => import('@/views/tools/Diff.vue'),
        meta: { requiresAuth: true, permission: 'tool:use', title: 'tools.diff.title' }
      },
      {
        path: 'tools/string',
        name: 'tool-string',
        component: () => import('@/views/tools/StringConverter.vue'),
        meta: { requiresAuth: true, permission: 'tool:use', title: 'tools.string.title' }
      },
      {
        path: 'tools/crypto',
        name: 'tool-crypto',
        component: () => import('@/views/tools/Crypto.vue'),
        meta: { requiresAuth: true, permission: 'tool:use', title: 'tools.crypto.title' }
      }
    ]
  },
  {
    path: '/:pathMatch(.*)*',
    component: () => import('@/views/NotFound.vue'),
    meta: { publicRoute: true }
  }
]

export const router = createRouter({
  history: createWebHashHistory(),
  routes
})

router.beforeEach(async (to, _from, next) => {
  const auth = useAuthStore()
  if (to.meta.title) {
    // best-effort: update document title
  }
  if (to.meta.publicRoute) return next()

  if (!auth.isAuthenticated) {
    return next({ name: 'login', query: { redirect: to.fullPath } })
  }
  if (to.meta.permission && !auth.hasPermission(to.meta.permission)) {
    return next({ name: 'dashboard' })
  }
  // Ensure locale store has the active locale (themes + locales were fetched at login,
  // but a refresh should re-hydrate).
  const localeStore = useLocaleStore()
  if (!localeStore.active) await localeStore.hydrate()
  next()
})