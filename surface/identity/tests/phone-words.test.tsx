/**
 * What a phone needs that is not layout: facts a finger can read without a hover. Two of the same name are told apart
 * in the row, and every draft cell names its column for the card a narrow screen makes of it. The card layout itself
 * is CSS under a 600 px media query, which jsdom does not apply; it is not proven here.
 */
import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import { $$, mount, unmountAll } from './harness';
import { ADA, DIRECTORY, SCRIBE, SERVICE, ok, reportingTo } from './fixtures';
import type { PeopleView } from '../src/generated';

beforeEach(() => sessionStorage.clear());
afterEach(() => unmountAll());
const holders = () => $$('td[data-col="Holder"]').map((cell) => cell.textContent);

describe('A phone reads what a hover would have said', () => {
  it('says which of two agents of the same name holds a grant, in the row', async () => {
    const twin = 'agent-' + 'e'.repeat(32);
    const people: PeopleView = { ...DIRECTORY, people: [{ ...DIRECTORY.people[0], agents: [...DIRECTORY.people[0].agents] },
      { id: 'person-' + 'd'.repeat(32), display_name: 'Dee (test person)', state: 'active', agents: [{ id: twin, display_name: 'Scribe', state: 'active', ...reportingTo({ id: 'person-' + 'd'.repeat(32), display_name: 'Dee (test person)', state: 'active' }) }] }] };
    await mount('#/access', { ...SERVICE, '/directory/people': ok(people) });
    expect(holders()).toContain('Scribeagent of Ada (test person), A/' + SCRIBE.slice(6, 14));
  });

  it('says nothing more when a name is unique', async () => {
    await mount('#/access', SERVICE);
    expect(holders()).toContain('Scribe');
    expect(holders()).toContain('Ada (test person)');
    expect($$('.g-which')).toHaveLength(0);
  });

  it('names every cell of a draft by its column', async () => {
    const draft = { id: 'op-' + '7'.repeat(32), agent: { id: SCRIBE, display_name: 'Scribe' }, responsible: { id: ADA, display_name: 'Ada (test person)', state: 'active' },
      target: { kind: 'project', id: 'identity', action: 'edit' }, method: 'POST', path: '/grants', body: '{}', note: 'n', created_at: 1790000000, creation_hash: 'ab'.repeat(32), state: 'waiting' };
    await mount('#/access/drafts', { ...SERVICE, '/drafts?state=waiting': ok({ drafts: [draft] }) });
    const row = $$('tr[data-draft] td');
    expect(row.map((cell) => cell.getAttribute('data-label'))).toEqual(['Agent', 'Wants to', 'Note', 'Asked', 'Decision']);
  });
});
