import { defineConfig } from 'vitest/config';

// R7's browser acceptance, run against the standalone identity server by
// crates/lys-identity-server/tests/why_acceptance.rs, which starts the
// server over a disposable SpiceDB, sets up the fixture and names both in
// LYS_WHY_ACCEPTANCE. The unit configuration in vite.config.ts never runs it.
export default defineConfig({
  esbuild: { jsx: 'automatic' },
  test: { environment: 'jsdom', include: ['tests/acceptance/**/*.spec.ts'], setupFiles: ['tests/setup.ts'] },
});
