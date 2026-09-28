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
  // `tests/**/*.spec.ts` is the acceptance shape: the same jsdom runner, kept
  // under its own name so an acceptance is not mistaken for a unit test.
  test: { environment: 'jsdom', include: ['tests/**/*.test.tsx', 'tests/acceptance/grants.spec.ts'], setupFiles: ['tests/setup.ts'] },
});
