import { defineStore } from 'pinia'

interface State {
  open: boolean
}

export const usePaletteStore = defineStore('palette', {
  state: (): State => ({ open: false }),
  actions: {
    show() { this.open = true },
    hide() { this.open = false },
    toggle() { this.open = !this.open }
  }
})