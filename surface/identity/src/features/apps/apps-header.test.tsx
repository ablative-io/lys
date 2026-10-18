// Apps has its own place on the rail, so its page is headed Apps, with the one page header every page has and no Settings tabs.
import { expect, it } from 'vitest';
import { mount } from '../../../tests/harness';
import { SERVICE, ok } from '../../../tests/fixtures';

it('heads the page Apps in the one page header, with no Settings tabs under it', async () => {
  await mount('#/apps', { ...SERVICE, '/apps': () => ok({ apps: [] }) });
  const headers = document.querySelectorAll('#screen .page-header');
  expect(headers).toHaveLength(1);
  expect(headers[0].querySelector('h1')?.textContent).toBe('Apps');
  expect(document.querySelectorAll('#screen h1')).toHaveLength(1);
  expect(document.querySelector('nav[aria-label="Settings views"]')).toBeNull();
  expect(document.querySelector('#rail [data-nav="apps"] .lbl')?.textContent).toBe('Apps');
});
