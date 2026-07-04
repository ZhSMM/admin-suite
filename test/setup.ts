// Global setup for vitest.  Runs before every test file.
//
// vue-i18n touches DOM in some paths; happy-dom provides a fake window/
// document.  We also stub out `localStorage` if the env doesn't have one.

if (typeof window !== 'undefined' && !window.matchMedia) {
  window.matchMedia = (query: string) => ({
    matches: false,
    media: query,
    onchange: null,
    addListener: () => {},
    removeListener: () => {},
    addEventListener: () => {},
    removeEventListener: () => {},
    dispatchEvent: () => false
  })
}