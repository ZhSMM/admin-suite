import { defineStore } from 'pinia'
import { menusApi, type Menu, type MenuNode } from '@/api/roles'
import { useAuthStore } from './auth'

interface State {
  menus: Menu[]
  loading: boolean
}

export const useMenuStore = defineStore('menu', {
  state: (): State => ({ menus: [], loading: false }),

  getters: {
    tree(): MenuNode[] {
      const items = [...this.menus].sort(
        (a, b) => a.sort_order - b.sort_order || a.code.localeCompare(b.code)
      )
      const map = new Map<string, MenuNode>()
      items.forEach((m) => map.set(m.id, { ...m, children: [] }))
      const roots: MenuNode[] = []
      items.forEach((m) => {
        const node = map.get(m.id)!
        if (m.parent_id && map.has(m.parent_id)) {
          map.get(m.parent_id)!.children.push(node)
        } else {
          roots.push(node)
        }
      })
      return roots
    }
  },

  actions: {
    async load() {
      const auth = useAuthStore()
      if (!auth.token) return
      this.loading = true
      try {
        // Login already returned menus; if user changes role we'd refetch here.
        // For now, store uses login data and this is a refresh path.
        // Call tree endpoint to refresh.
        const tree = await menusApi.tree(auth.token)
        // Flatten back into Menu[] (we keep `parent_id` info).
        const out: Menu[] = []
        const walk = (nodes: MenuNode[]) => {
          for (const n of nodes) {
            const { children, ...rest } = n
            out.push(rest as Menu)
            walk(children)
          }
        }
        walk(tree)
        this.menus = out
      } finally {
        this.loading = false
      }
    },

    setFromLogin(menus: Menu[]) {
      this.menus = menus
    }
  }
})