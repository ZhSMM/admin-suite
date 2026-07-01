/**
 * Apply a theme by setting CSS custom properties on `:root`.
 * This is the runtime analogue of a CSS preprocessor theme — Element Plus
 * reads many of these via its own theming variables, so they line up.
 */
import type { Resource } from '@/api/resources'

interface ThemePayload {
  id: string
  label: string
  isDark: boolean
  tokens: Record<string, string>
}

const FALLBACK_THEME: ThemePayload = {
  id: 'light',
  label: 'Light',
  isDark: false,
  tokens: {
    '--bg-primary': '#ffffff',
    '--bg-secondary': '#f5f7fa',
    '--bg-sidebar': '#001529',
    '--bg-sidebar-text': '#ffffff',
    '--text-primary': '#1f2329',
    '--text-secondary': '#646a73',
    '--text-inverse': '#ffffff',
    '--border-color': '#e5e6eb',
    '--primary-color': '#409eff',
    '--success-color': '#67c23a',
    '--warning-color': '#e6a23c',
    '--danger-color': '#f56c6c',
    '--info-color': '#909399',
    '--shadow-sm': '0 1px 2px rgba(0,0,0,0.06)',
    '--shadow-md': '0 2px 8px rgba(0,0,0,0.10)'
  }
}

function parseTheme(r: Resource | null): ThemePayload {
  if (!r) return FALLBACK_THEME
  try {
    const p = JSON.parse(r.content) as ThemePayload
    if (!p.tokens || typeof p.tokens !== 'object') return FALLBACK_THEME
    return p
  } catch {
    return FALLBACK_THEME
  }
}

export function applyTheme(r: Resource | null) {
  const theme = parseTheme(r)
  const root = document.documentElement
  // Toggle dark class on <html> so Element Plus picks up its dark variant.
  root.classList.toggle('dark', !!theme.isDark)
  root.dataset.theme = theme.id
  for (const [k, v] of Object.entries(theme.tokens)) {
    root.style.setProperty(k, String(v))
  }
  // Element Plus CSS variable bridge — pass through our semantic tokens.
  root.style.setProperty('--el-color-primary', theme.tokens['--primary-color'] || '#409eff')
  root.style.setProperty('--el-color-success', theme.tokens['--success-color'] || '#67c23a')
  root.style.setProperty('--el-color-warning', theme.tokens['--warning-color'] || '#e6a23c')
  root.style.setProperty('--el-color-danger', theme.tokens['--danger-color'] || '#f56c6c')
  root.style.setProperty('--el-color-info', theme.tokens['--info-color'] || '#909399')
  root.style.setProperty('--el-bg-color', theme.tokens['--bg-primary'] || '#ffffff')
  root.style.setProperty(
    '--el-bg-color-page',
    theme.tokens['--bg-secondary'] || '#f5f7fa'
  )
  root.style.setProperty('--el-text-color-primary', theme.tokens['--text-primary'] || '#1f2329')
  root.style.setProperty(
    '--el-text-color-regular',
    theme.tokens['--text-secondary'] || '#646a73'
  )
  root.style.setProperty('--el-border-color', theme.tokens['--border-color'] || '#e5e6eb')
}