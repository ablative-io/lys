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
  // No test or hook is bounded by a clock (CLAUDE.md, No time limits): 0 turns
  // Vitest's default 5000 ms and 10000 ms bounds off, so a slow run is found
  // at its cause and never failed by a timer.
  test: {
    environment: 'jsdom',
    include: ['tests/**/*.test.tsx'],
    setupFiles: ['tests/setup.ts'],
    testTimeout: 0,
    hookTimeout: 0,
  },
});
