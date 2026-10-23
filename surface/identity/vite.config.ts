import { defineConfig } from 'vitest/config';

// The page reads the identity service under /api on its own origin, so its
// session cookie is first-party and no cross-origin rule is needed. The proxy
// strips /api because a service started without screens serves its routes at
// the root (crates/lys-identity-server/src/routes_startup.rs, served_routes:
// `None => api`), as the default 8411 dev service does. An installed Lys serves
// screens, so its routes are under /api (surface.rs, `nest_service("/api", api)`):
// against one, pass the /api in the address, e.g.
// LYS_IDENTITY_SERVICE=http://127.0.0.1:8490/api.
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
  // No time limits (CLAUDE.md): vitest's default per-test and per-hook clocks
  // of five and ten seconds are switched off, so a test under a loaded machine
  // is judged by what it asserts, never by how long it took. A test that
  // hangs is found by its signal, never by a clock.
  test: {
    environment: 'jsdom',
    include: ['src/features/provisioning/*.test.tsx', 'tests/**/*.test.tsx', 'src/features/runtime/*.test.tsx', 'src/features/budgets/*.test.tsx', 'src/features/network-self-service.test.tsx',
      'src/features/file/*.test.tsx', 'src/features/people/*.test.tsx', 'src/features/apps/*.test.tsx', 'src/features/network/*.test.tsx', 'src/features/secrets/*.test.tsx',
      'src/features/connections/*.test.tsx', 'src/features/proxy/*.test.tsx', 'src/features/dashboard/*.test.tsx', 'src/features/usage/*.test.tsx',
      'src/features/requests/*.test.tsx',
      'tests/acceptance/grants.spec.ts', 'tests/acceptance/agent-control.spec.ts'],
    setupFiles: ['tests/setup.ts'],
    testTimeout: 0,
    hookTimeout: 0,
  },
});
