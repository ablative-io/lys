/**
 * ACCESS-009 R4: Add person and Give access are offered from the caller's own grants on their organisation, which
 * the Access and People heads name; a direct entry without the right is told which right, and given no form.
 */
import { beforeEach, describe, expect, it } from 'vitest';
import { $, mount, settle, text } from './harness';
import { FOUNDING_G, GRANTS, SERVICE, ok, refused } from './fixtures';
import type { Route } from './fixtures';

beforeEach(() => sessionStorage.clear());

/** Ada within Example organisation but holding nothing on it: the founding grant is not hers, and the wider directory is refused her. */
const WITHOUT: Record<string, Route> = {
  ...SERVICE,
  '/grants': ok({ grants: GRANTS.filter((grant) => grant.id !== FOUNDING_G), revision: 8 }),
  '/directory/people': refused(403, 'NotAdmitted', 'the wider directory is for its administrators'),
};

describe('Give access from organisation.grant', () => {
  it('names the organisation on Access and offers Give access to its holder', async () => {
    await mount('#/access');
    expect($('.head .sub')?.textContent ?? text()).toContain('Example organisation.');
    expect($('a[href="#/access/issue"]')?.textContent).toBe('Give access');
  });

  it('offers nothing without it, and a direct entry names the right and the organisation, with no form', async () => {
    const { posted } = await mount('#/access', WITHOUT);
    expect($('a[href="#/access/issue"]')).toBeNull();
    location.hash = '#/access/issue';
    await settle();
    expect($('form[aria-label="Give access"]')).toBeNull();
    const panel = $('.act-panel.issue-root');
    expect(panel?.querySelectorAll('input, select, textarea, button[type="submit"]')).toHaveLength(0);
    expect(panel?.querySelector('.refusal-name')?.textContent).toContain('RootAuthorityRefused');
    expect(panel?.textContent).toContain('organisation.grant on Example organisation');
    expect(posted).toHaveLength(0);
  });
});

describe('Add person from organisation.admit', () => {
  it('offers Add person to its holder, and the form names the organisation they join', async () => {
    await mount('#/people');
    expect($('a[href="#/people/new"]')?.textContent).toBe('Add person');
    location.hash = '#/people/new';
    await settle();
    expect($('form[aria-label="Add a person"]')).not.toBeNull();
    expect(text()).toContain('They join Example organisation.');
  });

  it('offers nothing without it, and a direct entry names the right, with no form and nothing sent', async () => {
    const { posted } = await mount('#/people', WITHOUT);
    expect($('a[href="#/people/new"]')).toBeNull();
    location.hash = '#/people/new';
    await settle();
    expect($('form[aria-label="Add a person"]')).toBeNull();
    expect(text()).toContain('needs organisation.admit there, which you do not hold');
    expect(text()).toContain('Example organisation');
    expect(posted).toHaveLength(0);
  });
});
