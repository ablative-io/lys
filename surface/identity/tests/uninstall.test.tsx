/** Uninstall on the administrator's account screen: data kept unless ticked, and what is lost named first. */
import { describe, expect, it } from 'vitest';
import { $, $$, click, mount, text, unreachable } from './harness';
import { SERVICE, ok, refused } from './fixtures';

const PLAN = {
  data_folder: '/Users/ada/Library/Application Support/lys/identity',
  lost: [
    "every person's sign-in account and password",
    'the people, agents and teams in the directory',
    'every permission and role that was given',
    'the signed record of every change',
    'every secret Lys keeps',
  ],
};

const ADMIN = { ...SERVICE, '/uninstall': ok(PLAN), 'POST /uninstall': ok({ uninstalling: true }) };

describe('Uninstall', () => {
  it('keeps the data folder unless the person ticks to remove it', async () => {
    const { posted } = await mount('#/me', ADMIN);
    expect($('#uninstall h2')?.textContent).toBe('Uninstall Lys');
    expect(unreachable()).toEqual([]);
    await click($('[data-act="uninstall-open"]'));
    expect(text()).toContain('installing Lys again signs the same people in');
    await click($('[data-act="uninstall"]'));
    expect(posted).toEqual([{ path: '/uninstall', body: { remove_data: false, confirmed: false } }]);
    expect($('#uninstalling')?.textContent).toContain('keeping your data folder');
  });

  it('names everything removing the data loses before it can be asked for', async () => {
    const { posted } = await mount('#/me', ADMIN);
    await click($('[data-act="uninstall-open"]'));
    await click($('input[name="remove_data"]'));
    const lost = $$('#uninstall-lost li').map((item) => item.textContent);
    expect(lost).toEqual(PLAN.lost);
    expect($('#uninstall-lost')?.textContent).toContain(PLAN.data_folder);
    const button = $('[data-act="uninstall"]');
    expect(button?.hasAttribute('disabled')).toBe(true);
    await click(button);
    expect(posted).toEqual([]);
    await click($('input[name="understood"]'));
    expect(button?.hasAttribute('disabled')).toBe(false);
    await click($('[data-act="uninstall"]'));
    expect(posted).toEqual([{ path: '/uninstall', body: { remove_data: true, confirmed: true } }]);
    expect($('#uninstalling')?.textContent).toContain('removing its data folder');
  });

  it('shows nothing to someone who is not the administrator', async () => {
    await mount('#/me', { ...SERVICE, '/uninstall': refused(403, 'NotAdmitted', 'only the configured administrator may do this in step 1') });
    expect($('#uninstall')).toBeNull();
    expect(text()).not.toContain('Uninstall Lys');
  });

  it('names why an install made from the command line cannot be uninstalled here', async () => {
    await mount('#/me', {
      ...SERVICE,
      '/uninstall': refused(503, 'UninstallUnavailable', 'UninstallUnavailable: this install keeps no uninstall helper'),
    });
    expect($('#uninstall .why-not b')?.textContent).toBe('UninstallUnavailable');
    expect($$('#uninstall button')).toHaveLength(0);
  });

  it('names a refused uninstall and leaves the control to try again', async () => {
    await mount('#/me', { ...ADMIN, 'POST /uninstall': refused(503, 'UninstallUnavailable', 'UninstallUnavailable: the helper could not start') });
    await click($('[data-act="uninstall-open"]'));
    await click($('[data-act="uninstall"]'));
    expect($('#uninstall [role="alert"] b')?.textContent).toBe('UninstallUnavailable');
    expect($('#uninstalling')).toBeNull();
    expect($('[data-act="uninstall"]')?.hasAttribute('disabled')).toBe(false);
  });
});
