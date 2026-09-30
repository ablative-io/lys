import { act } from 'react';
import type { ReactNode } from 'react';
import { createRoot } from 'react-dom/client';
import type { Root } from 'react-dom/client';
import { MemoryRouter } from 'react-router';
import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import { Teams } from '../src/features/teams/Teams';
import { ServiceAccounts } from '../src/features/service-accounts/ServiceAccounts';
import { PersonalBudgets } from '../src/features/file/PersonalBudgets';
import { AgentPolicy } from '../src/features/file/AgentPolicy';
import { AgentCredentials } from '../src/features/file/AgentCredentials';
import { StopReceipt } from '../src/features/file/EmergencyStop';
import { RecordedForm, TextField } from '../src/features/people/RecordedForm';
import { serve } from './harness';
import { ADA, ME, SCRIBE, SERVICE, ok, refused } from './fixtures';
import type { Route } from './fixtures';

let root: Root | undefined;
beforeEach(() => sessionStorage.clear());
afterEach(() => { if (root) act(() => root?.unmount()); root = undefined; });

async function show(page: ReactNode, more: Record<string, Route> = {}) {
  const posted: { path: string; body: unknown }[] = [];
  const requests = serve({ ...SERVICE, ...more }, posted);
  const element = document.createElement('div');
  document.body.append(element);
  root = createRoot(element);
  await act(async () => root?.render(<MemoryRouter>{page}</MemoryRouter>));
  return { posted, requests };
}

function button(words: string): HTMLButtonElement {
  const found = [...document.querySelectorAll('button')].find((entry) => entry.textContent === words);
  if (!found) throw new Error('Missing button: ' + words);
  return found;
}

async function click(words: string) { await act(async () => button(words).click()); }

async function fill(selector: string, value: string) {
  const input = document.querySelector(selector);
  if (!(input instanceof HTMLInputElement)) throw new Error('Missing field: ' + selector);
  await act(async () => {
    Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, 'value')?.set?.call(input, value);
    input.dispatchEvent(new Event('input', { bubbles: true }));
  });
}

function visibleWords(element: Element = document.body): string {
  const copy = element.cloneNode(true) as Element;
  for (const details of copy.querySelectorAll('details:not([open])')) {
    details.replaceChildren(details.querySelector('summary')?.cloneNode(true) ?? document.createTextNode(''));
  }
  return copy.textContent ?? '';
}

const team = { id: 'op-' + 'a'.repeat(32), owner: ADA, name: 'Delivery', description: 'Ship work', members: [SCRIBE], state: 'active', created_by: ME.signed_in, created_at: 1790000000, retired_at: null };
const account = { id: 'op-' + 'b'.repeat(32), owner: ADA, name: 'Calendar', description: 'Appointments', state: 'active', created_by: ME.signed_in, created_at: 1790000000, retired_by: null, retired_at: null };

describe('Directory words and write answers', () => {
  it('shows the team returned by creation without re-reading the old list', async () => {
    const world = await show(<Teams />, { '/teams': ok({ teams: [team] }), 'POST /teams': (body) => {
      const asked = body as Record<string, unknown>;
      return ok({ ...team, id: asked.operation, name: asked.name, description: asked.description, members: [], recorded: { operation: asked.operation, act: 'created', member: null, by: ME.signed_in, at: 1790000001 } });
    } });
    await click('+ Create a team');
    await fill('form[aria-label="Create team"] input', 'Reviewers');
    await click('Create team');
    expect(world.posted).toHaveLength(1);
    expect(world.requests.filter((path) => path === '/teams')).toHaveLength(1);
    expect(document.querySelectorAll('.team-tree [data-team]').length).toBe(2);
    expect(visibleWords()).toContain('Reviewers');
    expect(visibleWords()).not.toContain(ADA);
  });

  it('shows account retirement from its answer, with the owner name', async () => {
    const world = await show(<ServiceAccounts />, { '/service-accounts': ok({ scope: 'personal', service_accounts: [account] }), ['POST /service-accounts/' + account.id + '/retire']: ok({ ...account, state: 'retired', retired_at: 1790000001 }) });
    expect(visibleWords()).toContain('Ada (test person)');
    expect(visibleWords()).not.toContain(ADA);
    await click('Retire this service account'); await click('Yes, retire Calendar');
    expect(world.posted).toHaveLength(1);
    expect(world.requests.filter((path) => path === '/service-accounts')).toHaveLength(1);
    expect(visibleWords()).toContain('retired');
  });

  it('uses the confirmed budget answer and leaves other pending measures alone', async () => {
    const path = '/budgets/person/' + ADA;
    const effective = { holder: { kind: 'person', id: ADA }, measure: 'tokens', limit: 100, period: null, act: 'stop', version: 1, by: 'administrator', at: 1 };
    const requested = { ...effective, limit: 200, version: 2 };
    const context = { ...effective, measure: 'context_percent', limit: 80 };
    const world = await show(<PersonalBudgets id={ADA} name="Ada" />, { [path]: ok({ holder: effective.holder, budgets: [requested, context], unconfirmed: [{ requested, effective, reason: 'Waiting for confirmation' }, { requested: { ...context, version: 2 }, effective: context, reason: 'Waiting for confirmation' }] }), ['POST ' + path + '/confirm']: ok({ ...requested, version: 3 }) });
    await click('Apply the new limit of 200 tokens');
    expect(world.requests.filter((entry) => entry === path)).toHaveLength(1);
    expect(document.querySelector('[aria-label="Pending tokens"]')).toBeNull();
    expect(document.querySelector('[aria-label="Pending context_percent"]')).not.toBeNull();
    expect(visibleWords()).toContain('200');
    expect(visibleWords()).toContain('Enforced budget');
  });

  it('shows the saved policy response without a second read', async () => {
    const path = '/agents/' + SCRIBE + '/policy';
    const initial = { agent: SCRIBE, policy: null, digest: null, applies: 'applies on the agent’s next launch' };
    const world = await show(<AgentPolicy id={SCRIBE} />, { [path]: ok(initial), ['POST ' + path]: ok({ ...initial, policy: { agent: SCRIBE, version: 1, rules: [] }, digest: 'digest-1' }) });
    await click('Save rules for the next start');
    expect(world.posted).toEqual([{ path, body: { version: 0, rules: [] } }]);
    expect(world.requests.filter((entry) => entry === path)).toHaveLength(1);
    expect(visibleWords()).toContain('Version 1');
    expect(visibleWords()).not.toContain('digest-1');
  });

  it('explains a suspended identity without calling its process stopped or displaying identifiers', async () => {
    await show(<StopReceipt answer={{ agent: SCRIBE, operation: 'op-stop', state: 'suspended', by: ADA, at: 1790000000, reason: 'Check access', certificates_withdrawn: ['certificate-1'], credentials_ended: null, credentials_refused: 'BrokerUnavailable', sessions_asked: ['session-1'] }} />);
    expect(visibleWords()).toContain('The agent’s access is suspended.');
    expect(visibleWords()).not.toContain('Stopped:');
    expect(visibleWords()).not.toContain('certificate-1');
    expect(visibleWords()).not.toContain('session-1');
    expect(visibleWords()).toContain('Credentials tab');
  });

  it('explains a failed credential read and keeps its code in details', async () => {
    await show(<AgentCredentials id={SCRIBE} />, { ['/secrets/handles?holder=' + SCRIBE]: refused(503, 'SecretsUnavailable', 'the broker cannot answer') });
    expect(visibleWords()).toContain('Ask the administrator');
    expect(visibleWords()).not.toContain('SecretsUnavailable');
    expect(document.body.textContent).toContain('SecretsUnavailable');
  });

  it('keeps uncertain registration recoverable without exposing its request identifier', async () => {
    await show(<RecordedForm name="register-person" title="Register a person" done={() => undefined} change={(data) => ({ path: '/people', body: { display_name: String(data.get('display_name')) } })}><TextField name="display_name" label="Full name" /></RecordedForm>, { 'POST /people': refused(503, 'StorageUncertain', 'write outcome unknown') });
    await fill('input[name="display_name"]', 'New person');
    await act(async () => document.querySelector('form')?.dispatchEvent(new Event('submit', { bubbles: true, cancelable: true })));
    expect(visibleWords()).toContain('Check whether Lys saved it');
    expect(visibleWords()).not.toContain('StorageUncertain');
    expect(visibleWords()).not.toMatch(/op-[0-9a-f]{32}/);
    expect(sessionStorage.getItem('lys.pending.register-person')).toContain('New person');
  });

  it('shows one heading when a registration tab already names its form', async () => {
    await show(<RecordedForm name="register-person" title="Register a person" heading="" done={() => undefined} change={() => ({ path: '/people', body: {} })}><TextField name="display_name" label="Full name" /></RecordedForm>);
    expect(document.querySelectorAll('h2')).toHaveLength(0);
  });
});
