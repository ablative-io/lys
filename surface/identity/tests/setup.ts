import { afterEach, beforeEach, vi } from 'vitest';
import { unmountAll } from './harness';

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

beforeEach(() => {
  localStorage.clear();
  history.replaceState(null, '', '/');
});

afterEach(() => {
  unmountAll();
  document.body.innerHTML = '';
  vi.unstubAllGlobals();
  vi.restoreAllMocks();
});
