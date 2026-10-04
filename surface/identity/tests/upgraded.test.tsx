/** A tab left open across an upgrade says so once the change feed answers again with the service on a new build. */
import { describe, expect, it, vi } from 'vitest';
import { page } from '../src/shell/Upgraded';
import { $, click, mount, settle, text } from './harness';
import { BUILD, SERVICE, ok } from './fixtures';
import type { Answer, Route } from './fixtures';

const NOTICE = 'Lys was upgraded. Reload to use the new screens.';
const NEWER = 'fedcba9876543210fedcba9876543210fedcba98';
const first = 'op-' + '0'.repeat(32);

/** The service, with the build it answers at /authority and the feed's next answer held until `answer` is called. */
function service() {
  let build = BUILD;
  let answer: (generation: string) => void = () => { throw new Error('the feed was not asked again'); };
  const next = new Promise<Answer>((resolve) => { answer = (generation) => resolve(ok({ generation })); });
  const routes: Record<string, Route> = {
    ...SERVICE,
    '/authority': () => ok({ authority: 'Step 1 of the directory has one administrator.', build }),
    ['/changes?after=' + first]: (() => next) as unknown as Route,
  };
  return { routes, upgrade: (to: string) => { build = to; }, answer };
}

describe('Upgraded', () => {
  it('says nothing while the service answers the build this page first read', async () => {
    const { routes, answer } = service();
    const { requests } = await mount('#/me', routes);
    expect(requests).toContain('/authority');
    answer('op-' + '1'.repeat(32));
    await settle();
    expect(requests.filter((path) => path === '/authority').length).toBeGreaterThan(1);
    expect(text()).not.toContain(NOTICE);
    expect($('.upgraded')).toBeNull();
  });

  it('says the service was upgraded, with a Reload button, once the feed answers on a new build', async () => {
    const { routes, upgrade, answer } = service();
    await mount('#/me', routes);
    expect($('.upgraded')).toBeNull();
    upgrade(NEWER);
    answer('op-' + '1'.repeat(32));
    await settle();
    expect($('.upgraded')?.textContent).toBe(NOTICE + 'Reload');
    expect(text().split(NOTICE)).toHaveLength(2);
    expect($('.upgraded button')?.textContent).toBe('Reload');
  });

  it('reloads the page when Reload is pressed', async () => {
    const reload = vi.spyOn(page, 'reload').mockImplementation(() => undefined);
    const { routes, upgrade, answer } = service();
    await mount('#/me', routes);
    upgrade(NEWER);
    answer('op-' + '1'.repeat(32));
    await settle();
    expect(reload).not.toHaveBeenCalled();
    await click($('.upgraded button'));
    expect(reload).toHaveBeenCalledTimes(1);
  });
});
