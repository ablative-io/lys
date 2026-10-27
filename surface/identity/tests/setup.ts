import { afterEach, beforeEach, vi } from 'vitest';
import { unmountAll } from './harness';

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

/**
 * The clock every test reads: the fixtures' grants carry dated windows (one
 * ends at noon on 27 October 2026), so a test that read the wall clock went
 * red the moment a date passed. Each test starts at this fixed instant, the
 * day the fixtures were last confirmed, and a test that moves time does so
 * from here.
 */
export const FIXTURE_NOW = new Date(2026, 9, 26, 12, 0);

// Pinned before any test file loads, so a fixture that reads the clock as
// its module loads reads this instant too.
vi.setSystemTime(FIXTURE_NOW);

beforeEach(() => {
  vi.setSystemTime(FIXTURE_NOW);
  localStorage.clear();
  history.replaceState(null, '', '/');
});

afterEach(() => {
  unmountAll();
  document.body.innerHTML = '';
  vi.unstubAllGlobals();
  vi.restoreAllMocks();
});
