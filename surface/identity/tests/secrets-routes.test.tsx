/** Secrets navigation reaches each served route and never reads a credential value. */
import { describe, expect, it } from 'vitest';
import { act } from 'react';
import { $, mount, pick, text, unreachable, settle } from './harness';
import { SERVICE, ok } from './fixtures';

const secret = { name: 'Calendar', class: 'oauth', owner: 'Tom', sequence: 2, upstream: 'never-render-upstream', header: 'never-render-header' };
const routes = { ...SERVICE,
  '/secrets': ok({ secrets: [secret] }), '/secrets/grants': ok({ grants: [] }),
  '/secrets/audit': ok({ verified_by: 'key-not-for-screen', lines: [] }),
  '/secrets/settings?secret=Calendar': ok({ secret: 'Calendar', scope: 'team/Operations', recipients: 'people_only', value: 'never-render-secret-value' }),
};

describe('Secrets routes', () => {
  it.each([['entries', '/secrets'], ['grants', '/secrets/grants'], ['audit', '/secrets/audit']])('opens %s through its served endpoint', async (section, path) => {
    const { requests, posted } = await mount('#/secrets/' + section, routes);
    expect(requests).toContain(path);
    expect(posted).toHaveLength(0);
    expect($('nav[aria-label="Secrets views"] a[aria-current="page"]')?.getAttribute('href')).toBe('#/secrets/' + section);
    expect(text()).not.toContain('not built yet');
    expect(text()).not.toContain('never-render');
    expect(text()).not.toContain('key-not-for-screen');
    expect(unreachable()).toEqual([]);
  });

  it('requires a named selection before showing owner controls and makes no automatic change', async () => {
    const { posted } = await mount('#/secrets/manage', routes);
    expect(document.querySelectorAll('form')).toHaveLength(0);
    await pick(document, 'Find a secret', 'Cal', 'Calendar');
    expect(document.querySelectorAll('form')).toHaveLength(2);
    expect(posted).toHaveLength(0);
    expect(text()).toContain('Only its owner');
    expect(text()).toContain('Who it can be handed to');
    expect(text()).toContain('the team Operations');
    expect(text()).not.toContain('never-render-secret-value');
  });
});


it('sends the retained operation through the actual recipients API', async () => {
  const { posted } = await mount('#/secrets/manage?secret=Calendar', { ...routes,
    'POST /secrets/recipients': ok({ secret: 'Calendar', recipients: 'people_only', operation: 'wrong-operation-on-purpose', repeated: false }),
  });
  const form = [...document.querySelectorAll('form')].find((entry) => entry.textContent?.includes('Who it can be handed to'));
  if (!form) throw new Error('No recipients form');
  await act(async () => form.dispatchEvent(new Event('submit', { bubbles: true, cancelable: true })));
  await settle();
  expect(posted).toHaveLength(1);
  expect(posted[0]).toMatchObject({ body: { secret: 'Calendar', recipients: 'people_only', operation: expect.stringMatching(/^[A-Za-z0-9_-]{16,64}$/) } });
  expect(text()).toContain('UnconfirmedAnswer');
});
