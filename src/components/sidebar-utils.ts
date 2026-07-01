/**
 * Sidebar helpers — kept in a separate file so they can be unit-tested
 * without spinning up a Vue component.
 */
import type { MenuNode } from '@/api/roles'

export function resolveIndex(node: MenuNode): string {
  // Use the route path if present, otherwise fall back to code.
  return node.path || `/${node.code.replace(/\./g, '/')}`
}

/**
 * Element Plus icons are registered globally under PascalCase names.
 * The stored icon string is a kebab-case name from the seed (e.g. 'user-filled');
 * convert to PascalCase so we can look it up.
 */
export function iconName(stored: string | null | undefined): string | null {
  if (!stored) return null
  const parts = stored.split('-').filter(Boolean)
  if (parts.length === 0) return null
  return parts
    .map((p) => p.charAt(0).toUpperCase() + p.slice(1))
    .join('')
}