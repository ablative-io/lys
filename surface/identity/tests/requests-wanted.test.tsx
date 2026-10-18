/** A refusal's link (`#/requests?kind=…&id=…&action=…`) opens the Ask form asking for exactly what was refused. */
import { act } from 'react';
import { beforeEach, describe, expect, it } from 'vitest';
import { $, mount, settle } from './harness';
import { ADA, ME, SERVICE, ok } from './fixtures';
import type { Ask } from '../src/features/requests/contract';

beforeEach(() => sessionStorage.clear());
const model = { action_sentences: { read: 'Read this resource', write: 'Write to this resource' }, version: 1,
  relations: { reader: ['read'], editor: ['read', 'write'], 'only.write': ['write'] },
  kinds: { 'cambium.channel': { agents: [], relations: { member: ['read', 'post'], 'only.post': ['post'] } } } };
const kept = (asked: Ask) => ({ ...asked, id: asked.operation, asked_by: ADA, asked_by_name: ME.person.display_name, responsible: ME.person,
  actions: ['post'], asked_at: 1790000000, state: 'waiting', approvers: [ME.person], sources: [], decision: null });
const routes = { ...SERVICE, '/requests': ok({ requests: [] }), '/grants/model': ok(model),
  '/grants': ok({ revision: 1, grants: [{ resource: { kind: 'project', id: 'Lys' } }] }), 'POST /requests': (body: unknown) => ok(kept(body as Ask)) };
const form = () => {
  const element = $('form[aria-label="Ask for access"]');
  if (!element) throw new Error('Ask form missing');
  return element;
};
async function send(why: string) {
  const box = form().querySelector('textarea');
  if (!box) throw new Error('Why field missing');
  box.value = why;
  form().querySelector<HTMLInputElement>('input[name="no-expiry"]')?.click();
  await act(async () => { form().dispatchEvent(new Event('submit', { bubbles: true, cancelable: true })); });
  await settle();
}

describe('Asking for what a refusal named', () => {
  it('says an app thing she does not hold as a sentence, folds its filled fields, and sends exactly that', async () => {
    const m = await mount('#/requests?kind=cambium.channel&id=general&action=post', routes);
    expect(form().querySelector('.ask-wanted')?.textContent).toBe('You are asking to post on Channel general in Cambium. Say why, and someone who can give it will decide.');
    expect(form().querySelector('details.ask-what')?.hasAttribute('open')).toBe(false);
    expect(form().querySelector<HTMLInputElement>('input[name="kind"]')?.value).toBe('cambium.channel');
    expect(form().querySelector<HTMLInputElement>('input[name="resource"]')?.value).toBe('general');
    await send('To post the release notes');
    expect(m.posted.filter((p) => p.path === '/requests').map((p) => p.body)).toEqual([{ operation: expect.stringMatching(/^op-[0-9a-f]{32}$/),
      resource: { kind: 'cambium.channel', id: 'general' }, relation: 'only.post', ends_at: null, why: 'To post the release notes' }]);
  });

  it('picks a thing she already holds from her list and ticks the one action', async () => {
    const m = await mount('#/requests?kind=project&id=Lys&action=write', routes);
    expect(form().querySelector('.ask-wanted')?.textContent).toBe('You are asking to write to Project Lys. Say why, and someone who can give it will decide.');
    expect(form().querySelector<HTMLSelectElement>('select')?.value).toBe('0');
    expect(form().querySelector<HTMLInputElement>('input[name="action"][value="write"]')?.checked).toBe(true);
    await send('To fix the release');
    expect(m.posted.filter((p) => p.path === '/requests').map((p) => (p.body as Ask).relation)).toEqual(['only.write']);
  });

  it('leaves the fields open when no single relation carries the named action, so nothing required hides under a fold', async () => {
    await mount('#/requests?kind=cambium.channel&id=general&action=archive', routes);
    expect(form().querySelector('details.ask-what')).toBeNull();
    expect(form().querySelector<HTMLSelectElement>('select[name="relation"]')?.value).toBe('');
  });

  it('opens the plain form, with no sentence and no fold, without a link', async () => {
    await mount('#/requests?kind=project&id=Lys', routes);
    expect($('form[aria-label="Ask for access"]')?.querySelector('.ask-wanted') ?? null).toBeNull();
    expect($('details.ask-what')).toBeNull();
  });
});
