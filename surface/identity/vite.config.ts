import { defineConfig } from 'vitest/config';

// The identity service answers under /api on the page's own origin, so its
// session cookie is first-party and no cross-origin rule is needed.
const service = process.env.LYS_IDENTITY_SERVICE ?? 'http://127.0.0.1:8411';
const proxy = {
  '/api': { target: service, changeOrigin: false, rewrite: (path: string) => path.replace(/^\/api/, '') },
};

export default defineConfig({
  esbuild: { jsx: 'automatic' },
  server: { proxy },
  preview: { proxy },
  // No time limits (CLAUDE.md): vitest's default per-test and per-hook clocks
  // of five and ten seconds are switched off, so a test under a loaded machine
  // is judged by what it asserts, never by how long it took. A test that
  // hangs is found by its signal, never by a clock.
  test: {
    environment: 'jsdom',
    include: ['tests/**/*.test.tsx'],
    setupFiles: ['tests/setup.ts'],
    testTimeout: 0,
    hookTimeout: 0,
  },
});
