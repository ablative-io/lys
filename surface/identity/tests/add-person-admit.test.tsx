import { act } from 'react';
import { createRoot } from 'react-dom/client';
import type { Root } from 'react-dom/client';
import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import { App } from '../src/App';
import { ADA, BEA, MODEL, SERVICE, ok, refused } from './fixtures';
import type { Route } from './fixtures';
import { choose, serve, type } from './harness';

/** Adding a person is one act (POST /people/admit): their name, their email and their first grant, answered by one receipt naming each step's log leaf. */
let root: Root | null = null;
beforeEach(() => sessionStorage.clear());
afterEach(() => { const mounted = root; if (mounted) act(() => mounted.unmount()); root = null; });

async function form(routes: Record<string, Route>) {
  const posted: { path: string; body: unknown }[] = [];
  serve({ ...SERVICE, ...routes }, posted);
  history.replaceState(null, '', '/#/people/new');
  const container = document.createElement('div'); document.body.appendChild(container);
  const mounted = createRoot(container); root = mounted;
  await act(async () => { mounted.render(<App />); });
  const entry = document.querySelector('form');
  if (!entry) throw new Error('Missing add-person form');
  return { entry, posted };
}

async function fill(entry: HTMLFormElement) {
  await type(entry.querySelector('input[name="given_name"]'), 'Care');
  await type(entry.querySelector('input[name="family_name"]'), 'Helper');
  await type(entry.querySelector('input[name="email"]'), 'helper@example.test');
  await choose(entry.querySelector('select[name="kind"]'), 'workspace');
  await choose(entry.querySelector('select[name="resource"]'), 'ward-7');
  await choose(entry.querySelector('select[name="relation"]'), 'editor');
}

async function submit(entry: HTMLFormElement) {
  await act(async () => { entry.dispatchEvent(new Event('submit', { bubbles: true, cancelable: true })); });
}

function admitted(body: unknown) {
  if (!body || typeof body !== 'object' || !('operation' in body) || typeof body.operation !== 'string') throw new Error('Missing operation');
  return {
    person: BEA, subject: 'subject-helper', grant: 'grant-' + 'b'.repeat(32),
    letter: { letter: 'added', to: 'helper@example.test', sent: true, by: 'lys' },
    receipt: {
      operation: body.operation, person: BEA, failed: null,
      completed: ['register', 'account', 'bind', 'activate', 'grant', 'letter'],
      logged: [
        { step: 'register', log: 'directory', index: 12 },
        { step: 'bind', log: 'directory', index: 13 },
        { step: 'activate', log: 'directory', index: 14 },
        { step: 'grant', log: 'grants', index: 7 },
      ],
    },
  };
}

/** The grant model with Cambium's workspace kind as its approved schema declares it, and one such workspace Lys holds a grant on. */
const CAMBIUM_MODEL = { ...MODEL, kinds: { ...MODEL.kinds, 'cambium.workspace': { agents: [], relations: {
  administrator: ['configure', 'invite', 'post', 'read', 'seat_found'], member: ['edit_own', 'post', 'read', 'seat_found'],
  owner: ['configure', 'invite', 'ownership_transfer', 'post', 'read'], reader: ['read'] } } } };
const CAMBIUM_RESOURCES = ok({ kinds: ['cambium.workspace', 'workspace'], revision: 1, judged_at: 1790000000, resources: [
  { kind: 'cambium.workspace', id: 'ablative', standing: 1, ended: 0, holders: 1 },
  { kind: 'workspace', id: 'ward-7', standing: 2, ended: 0, holders: 2 },
] });

const options = (entry: HTMLFormElement) => [...entry.querySelectorAll('select[name="relation"] option')].map((option) => (option as HTMLOptionElement).value).filter(Boolean);

describe('Add a person to an app', () => {
  const value = (entry: HTMLFormElement) => (entry.querySelector('select[name="relation"]') as HTMLSelectElement).value;
  const DECLARED = { ...CAMBIUM_MODEL, kinds: { ...CAMBIUM_MODEL.kinds, 'cambium.workspace': { agents: [], first_grant: 'member', relations: {
    administrator: ['configure', 'invite', 'post', 'read', 'seat_found'], member: ['edit_own', 'post', 'read', 'seat_found'], reader: ['read'] } } } };

  it("preselects the app's declared first grant under As, says it is the app's usual first access, and leaves the choice open", async () => {
    const { entry } = await form({ '/grants/model': ok(DECLARED), '/resources': CAMBIUM_RESOURCES, '/apps': ok({ apps: [{ id: 'cambium', name: 'Cambium', state: 'approved' }] }) });
    await choose(entry.querySelector('select[name="kind"]'), 'cambium.workspace');
    expect(value(entry)).toBe('member');
    expect(entry.textContent).toContain("Cambium's usual first access");
    await choose(entry.querySelector('select[name="relation"]'), 'reader');
    expect(value(entry)).toBe('reader');
    expect(entry.textContent).not.toContain('usual first access');
  });

  it('preselects nothing and requires a choice when the app declares no first grant', async () => {
    const { entry } = await form({ '/grants/model': ok(CAMBIUM_MODEL), '/resources': CAMBIUM_RESOURCES });
    await choose(entry.querySelector('select[name="kind"]'), 'cambium.workspace');
    expect(value(entry)).toBe('');
    expect(entry.textContent).not.toContain('usual first access');
  });

  it("offers an app kind the app's own relations, never Lys's, and sends the chosen one by name", async () => {
    const { entry, posted } = await form({ '/grants/model': ok(CAMBIUM_MODEL), '/resources': CAMBIUM_RESOURCES, 'POST /people/admit': (body) => ok(admitted(body)) });
    await type(entry.querySelector('input[name="given_name"]'), 'Dean');
    await type(entry.querySelector('input[name="family_name"]'), 'Whiting');
    await type(entry.querySelector('input[name="email"]'), 'dean@example.test');
    await choose(entry.querySelector('select[name="kind"]'), 'cambium.workspace');
    await choose(entry.querySelector('select[name="resource"]'), 'ablative');
    expect(options(entry)).toEqual(['administrator', 'member', 'owner', 'reader']);
    const member = entry.querySelector('select[name="relation"] option[value="member"]');
    expect(member?.textContent).toBe('member: edit_own; post; read; seat_found');
    await choose(entry.querySelector('select[name="kind"]'), 'workspace');
    expect(options(entry)).toEqual(['editor', 'owner', 'viewer']);
    await choose(entry.querySelector('select[name="kind"]'), 'cambium.workspace');
    await choose(entry.querySelector('select[name="resource"]'), 'ablative');
    await choose(entry.querySelector('select[name="relation"]'), 'member');
    await submit(entry);
    expect(posted.map((call) => call.path)).toEqual(['/people/admit']);
    const sent = posted[0]?.body as Record<string, unknown>;
    expect(sent.grant).toEqual({ route: 'browser', resource: { kind: 'cambium.workspace', id: 'ablative' }, relation: 'member' });
  });
});

describe('Add a person in one act', () => {
  it('asks for the name, the email and the first grant, and sends one admit', async () => {
    const { entry, posted } = await form({ 'POST /people/admit': (body) => ok(admitted(body)) });
    await fill(entry);
    await submit(entry);
    expect(posted.map((call) => call.path)).toEqual(['/people/admit']);
    const sent = posted[0]?.body as Record<string, unknown>;
    expect(sent.display_name).toBe('Care Helper');
    expect(sent.given_name).toBe('Care');
    expect(sent.family_name).toBe('Helper');
    expect(sent.email).toBe('helper@example.test');
    expect(sent.grant).toEqual({ route: 'browser', resource: { kind: 'workspace', id: 'ward-7' }, relation: 'editor' });
    expect(sent.operation).toMatch(/^op-[0-9a-f]{32}$/);
  });

  it('shows the receipt with each step and its log leaf', async () => {
    const { entry } = await form({ 'POST /people/admit': (body) => ok(admitted(body)) });
    await fill(entry);
    await submit(entry);
    const receipt = document.querySelector('[aria-label="Receipt"]');
    expect(receipt).not.toBeNull();
    const text = receipt?.textContent ?? '';
    for (const words of ['directory log, entry 12', 'directory log, entry 13', 'directory log, entry 14', 'grants log, entry 7']) {
      expect(text).toContain(words);
    }
    expect(text).toContain('issuer account');
    expect(document.querySelector('a[href="#/file/' + BEA + '"]')).not.toBeNull();
    expect(sessionStorage.getItem('lys.add-person.' + ADA)).toBeNull();
  });

  it('says that Lys wrote to them at their email, and offers no invitation to copy', async () => {
    const { entry } = await form({ 'POST /people/admit': (body) => ok(admitted(body)) });
    await fill(entry);
    await submit(entry);
    expect(document.querySelector('[aria-label="Receipt"] p[role="status"]')?.textContent).toBe('Care Helper is added. Lys wrote to them at helper@example.test.');
    expect(document.body.textContent).not.toContain('Copy invite');
    expect(document.querySelector('[aria-label^="Copy the invitation"]')).toBeNull();
  });

  it('says the sign-in service wrote to them when it made their account', async () => {
    const byIssuer = (body: unknown) => ok({ ...admitted(body), letter: { letter: 'added', to: 'helper@example.test', sent: true, by: 'issuer' } });
    const { entry } = await form({ 'POST /people/admit': byIssuer });
    await fill(entry);
    await submit(entry);
    expect(document.querySelector('[aria-label="Receipt"] p[role="status"]')?.textContent).toBe('Care Helper is added. The sign-in service wrote to them at helper@example.test with their first sign-in.');
  });

  it('shows an admit whose letter was not sent as added but unwritten, with every step kept', async () => {
    const unsent = (body: unknown) => {
      const operation = (body as { operation: string }).operation;
      return { status: 503, body: {
        refusal: 'mail_not_sent', reason: 'mail_not_sent: the added letter to helper@example.test was not sent: permanent error (554): the sink refuses this letter; the act is recorded, ask again with the same operation to send it',
        receipt: { operation, person: BEA, completed: ['register', 'account', 'bind', 'activate', 'grant'], failed: 'letter',
          logged: [{ step: 'register', log: 'directory', index: 12 }, { step: 'bind', log: 'directory', index: 13 }, { step: 'activate', log: 'directory', index: 14 }, { step: 'grant', log: 'grants', index: 7 }] },
      } };
    };
    const { entry } = await form({ 'POST /people/admit': unsent });
    await fill(entry);
    await submit(entry);
    expect(document.querySelector('[aria-label="Receipt"]')).toBeNull();
    const receipt = document.querySelector('[aria-label="Refused receipt"]');
    const text = receipt?.textContent ?? '';
    expect(text).toContain('Added, but the letter was not sent. Lys refused while writing to them');
    expect(text).toContain('permanent error (554)');
    expect([...(receipt?.querySelectorAll(':scope > ul > li') ?? [])].map((leaf) => leaf.textContent)).toEqual(['Registered', 'Sign-in bound', 'Activated', 'First grant issued']);
  });

  it('confirms an answer whose letter is handed and shows the link to give them (DIRECTORY-112 R4)', async () => {
    const link = 'https://lys.example.test/#/added/xK9mQ2vLp8Rt4Wn7Yb3Zc6Fh1Jd5Sg0Ae2Uo4Ii8Qw';
    const { entry } = await form({ 'POST /people/admit': (body) => ok({ ...admitted(body), letter: { letter: 'added', to: 'helper@example.test', sent: false, by: 'lys', reason: 'Lys has no mail settings, so the link is handed here', handed: link } }) });
    await fill(entry);
    await submit(entry);
    expect(document.querySelector('[role="status"]')?.textContent).toBe('Care Helper is added. This Lys cannot send email, so give them this link; opening it signs them in once, and they set a password there.');
    const field = document.querySelector('input[aria-label="Their sign-in link"]') as HTMLInputElement | null;
    expect(field?.value).toBe(link);
    expect(field?.readOnly).toBe(true);
    const receipt = document.querySelector('[aria-label="Receipt"]');
    expect(receipt).not.toBeNull();
    expect(receipt?.textContent).toContain('Letter: handed to you here to give them; nothing was sent');
  });

  it('refuses an answer whose letter did not go as unconfirmed, keeping the request', async () => {
    const { entry } = await form({ 'POST /people/admit': (body) => ok({ ...admitted(body), letter: { letter: 'added', sent: false, by: 'lys', reason: 'mail_unconfigured' } }) });
    await fill(entry);
    await submit(entry);
    expect(document.querySelector('[aria-label="Receipt"]')).toBeNull();
    expect(entry.textContent).toContain('did not confirm');
  });

  it('refuses an answer whose receipt is not for this operation, keeping the request', async () => {
    const { entry } = await form({ 'POST /people/admit': (body) => ok({ ...admitted(body), receipt: { ...admitted(body).receipt, operation: 'op-' + '0'.repeat(32) } }) });
    await fill(entry);
    await submit(entry);
    expect(document.querySelector('[aria-label="Receipt"]')).toBeNull();
    expect(entry.textContent).toContain('did not confirm');
    expect(sessionStorage.length).toBeGreaterThan(0);
  });

  it('names a refused step in the refusal and adds nothing on screen', async () => {
    const { entry } = await form({ 'POST /people/admit': refused(400, 'RelationUnknown', 'gamma is not a relation') });
    await fill(entry);
    await submit(entry);
    expect(entry.textContent).toContain('gamma is not a relation');
    expect(document.querySelector('[aria-label="Receipt"]')).toBeNull();
  });

  it('shows a refused receipt: the refusal by name and reason, the step it stopped at, and each written step\'s log leaf', async () => {
    const stopped = (body: unknown) => {
      const operation = (body as { operation: string }).operation;
      return { status: 403, body: {
        refusal: 'WithheldFromAgents', reason: 'WithheldFromAgents: editor carries grant.delegate, which no agent may hold',
        receipt: { operation, person: BEA, completed: ['register', 'account', 'bind', 'activate'], failed: 'grant',
          logged: [{ step: 'register', log: 'directory', index: 12 }, { step: 'bind', log: 'directory', index: 13 }, { step: 'activate', log: 'directory', index: 14 }] },
      } };
    };
    const { entry } = await form({ 'POST /people/admit': stopped });
    await fill(entry);
    await submit(entry);
    expect(document.querySelector('[aria-label="Receipt"]')).toBeNull();
    const receipt = document.querySelector('[aria-label="Refused receipt"]');
    expect(receipt).not.toBeNull();
    expect(receipt?.querySelector('details .refusal-name')?.textContent).toContain('WithheldFromAgents');
    const text = receipt?.textContent ?? '';
    expect(text).toContain('no agent may hold');
    expect(text).toContain('Lys refused while giving their first access');
    // The steps already done are said in words; where each was recorded is folded under its own summary.
    expect([...(receipt?.querySelectorAll(':scope > ul > li') ?? [])].map((leaf) => leaf.textContent)).toEqual(['Registered', 'Sign-in bound', 'Activated']);
    expect([...(receipt?.querySelectorAll('details ul > li') ?? [])].map((leaf) => leaf.textContent)).toEqual([
      'Registered: directory log, entry 12', 'Sign-in bound: directory log, entry 13', 'Activated: directory log, entry 14',
    ]);
    // The refusal's name is never in the open sentence, only under Error details.
    expect(receipt?.querySelector(':scope > p')?.textContent).not.toContain('WithheldFromAgents');
  });

  it('does not take a refused receipt for another operation as this act\'s', async () => {
    const elsewhere = { status: 403, body: { refusal: 'IdentityNotActive', reason: 'not active',
      receipt: { operation: 'op-' + '0'.repeat(32), person: null, completed: [], failed: 'register', logged: [] } } };
    const { entry } = await form({ 'POST /people/admit': elsewhere });
    await fill(entry);
    await submit(entry);
    expect(document.querySelector('[aria-label="Refused receipt"]')).toBeNull();
    expect(entry.textContent).toContain('IdentityNotActive');
  });
  it('adds a person with no access when none is chosen, sending no grant, and says so', async () => {
    const noGrant = (body: unknown) => {
      const answer = admitted(body);
      return ok({ ...answer, grant: null, receipt: { ...answer.receipt, completed: ['register', 'account', 'bind', 'activate', 'letter'], logged: answer.receipt.logged.filter((leaf) => leaf.step !== 'grant') } });
    };
    const { entry, posted } = await form({ 'POST /people/admit': noGrant });
    expect((entry.querySelector('select[name="kind"]') as HTMLSelectElement).value).toBe('');
    expect(entry.querySelector('select[name="kind"] option[value=""]')?.textContent).toBe('No access yet');
    expect(entry.querySelector('select[name="resource"]')).toBeNull();
    expect(entry.textContent).toContain('Give them access to');
    await type(entry.querySelector('input[name="given_name"]'), 'Care');
    await type(entry.querySelector('input[name="family_name"]'), 'Helper');
    await type(entry.querySelector('input[name="email"]'), 'helper@example.test');
    expect(entry.querySelector('button[type="submit"]')?.hasAttribute('disabled')).toBe(false);
    await submit(entry);
    expect(posted.map((call) => call.path)).toEqual(['/people/admit']);
    expect('grant' in (posted[0]?.body as Record<string, unknown>)).toBe(false);
    const receipt = document.querySelector('[aria-label="Receipt"]');
    expect(receipt?.textContent).toContain('They have no access yet');
  });

  it('says resources and what they may do in words, never raw relation lists', async () => {
    const { entry } = await form({});
    await choose(entry.querySelector('select[name="kind"]'), 'workspace');
    const names = [...entry.querySelectorAll('select[name="resource"] option')].map((option) => option.textContent);
    expect(names.slice(1)).toEqual(['a workspace, ward-3', 'a workspace, ward-7']);
    const relations = [...entry.querySelectorAll('select[name="relation"] option')].map((option) => option.textContent ?? '');
    expect(relations.some((words) => words.includes(' · '))).toBe(false);
    expect(entry.textContent).toContain('As');
  });

  it('offers as a first grant only the kinds and resources Lys lists, never a person or an agent', async () => {
    const { entry } = await form({});
    const kinds = [...entry.querySelectorAll('select[name="kind"] option')].map((option) => option.getAttribute('value'));
    expect(kinds).toEqual(['', 'place', 'workspace']);
    expect(entry.querySelector('select[name="resource"]')).toBeNull();
    await choose(entry.querySelector('select[name="kind"]'), 'workspace');
    const ids = [...entry.querySelectorAll('select[name="resource"] option')].map((option) => option.getAttribute('value'));
    expect(ids).toEqual(['', 'ward-3', 'ward-7']);
    await choose(entry.querySelector('select[name="resource"]'), 'ward-3');
    await choose(entry.querySelector('select[name="kind"]'), 'place');
    expect((entry.querySelector('select[name="resource"]') as HTMLSelectElement).value).toBe('');
  });
});
