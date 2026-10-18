/** A kept change this tab cannot read back pauses sending, and Check with Lys settles it: never "ask an administrator", never stuck for good. */
import { act } from 'react';
import { createRoot } from 'react-dom/client';
import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import { $, click, mount, serve, settle, text, unmountAll } from './harness';
import { ADA, AGENT_MODEL, DIRECTORY, GRANTS, ME, ROOT_G, SERVICE, ok, refused } from './fixtures';
import { DecisionForm } from '../src/features/requests/DecisionForm';
import { ShellProvider } from '../src/shell/ShellContext';
import type { AccessRequest } from '../src/features/requests/contract';
import type { PeopleView } from '../src/generated';

beforeEach(() => sessionStorage.clear());
afterEach(() => unmountAll());
const OP = 'op-' + '7'.repeat(32);
const KEY = 'lys.pending.request.' + ADA;
const model = { action_sentences: { read: 'Read this resource' }, version: 1, relations: { reader: ['read'] }, kinds: {} };
const asked = (id: string, state: AccessRequest['state'] = 'waiting') => ({ id, asked_by: ADA, asked_by_name: ME.person.display_name, responsible: ME.person, resource: { kind: 'project', id: 'Lys' },
  relation: 'reader', actions: ['read'], why: 'x', ends_at: null, asked_at: 1790000000, state, approvers: [ME.person], sources: [], decision: null });
const routes = (requests: unknown[]) => ({ ...SERVICE, '/requests': ok({ requests }), '/grants/model': ok(model), '/grants': ok({ revision: 1, grants: [] }) });
const button = (label: string) => [...document.querySelectorAll('button')].find((each) => (each.getAttribute('aria-label') ?? each.textContent) === label) ?? null;
const askOpen = () => $('form[aria-label="Ask for access"] fieldset.ask-fields') as HTMLFieldSetElement | null;

describe('Check with Lys, on the Ask form', () => {
  const asking = (answer: unknown) => ({ ...routes([]), ['/operations/' + OP]: answer });

  it('asks Lys by the kept operation, says Lys recorded it with a link to it, and opens the form on Carry on without sending anything', async () => {
    sessionStorage.setItem(KEY, '{"operation":"' + OP + '", "resource": broken');
    const m = await mount('#/requests', asking(ok({ recorded: true, kind: 'request', id: OP })));
    expect(askOpen()?.disabled).toBe(true);
    expect(text()).not.toContain('administrator');
    await click(button('Check with Lys'));
    expect(text()).toContain('Lys recorded this: your request went through, so there is nothing to send again.');
    expect($('.stuck-record a[href="#/requests/' + OP + '"]')).not.toBeNull();
    await click(button('Carry on'));
    expect(sessionStorage.getItem(KEY)).toBeNull();
    expect(askOpen()?.disabled).toBe(false);
    expect(m.posted).toHaveLength(0);
  });

  it('says a change Lys has no record of did not go through, keeps what was typed, and clears only on her press', async () => {
    sessionStorage.setItem(KEY, '{"operation":"' + OP + '", "resource": broken');
    await mount('#/requests', asking(ok({ recorded: false })));
    await click(button('Check with Lys'));
    expect(text()).toContain('Lys has no record of your request, so it did not go through. Nothing you typed is lost: clear it and the form opens again.');
    expect(sessionStorage.getItem(KEY)).not.toBeNull();
    await click(button('Clear it and carry on'));
    expect(sessionStorage.getItem(KEY)).toBeNull();
    expect(askOpen()?.disabled).toBe(false);
  });

  it('cannot tell when Lys refuses to say, and never says it did not go through', async () => {
    sessionStorage.setItem(KEY, '{"operation":"' + OP + '", "resource": broken');
    await mount('#/requests', asking(refused(403, 'NotAdmitted', 'this operation was not sent by the signed-in caller')));
    await click(button('Check with Lys'));
    expect(text()).toContain('Lys cannot tell from what you may see whether this went through. Do not ask for it again.');
    expect(text()).not.toContain('did not go through');
    expect(button('Forget this kept copy')).not.toBeNull();
  });

  it('cannot tell when no operation is left to ask about, says not to ask again, and offers only to forget the copy', async () => {
    sessionStorage.setItem(KEY, '{broken');
    await mount('#/requests', routes([]));
    expect(text()).toContain('Lys cannot tell from what you may see whether this went through. Do not ask for it again.');
    expect(text()).not.toContain('did not go through');
    expect(button('Clear it and carry on')).toBeNull();
    await click(button('Forget this kept copy'));
    expect(sessionStorage.getItem(KEY)).toBeNull();
    expect(askOpen()?.disabled).toBe(false);
  });

  it('clears nothing when Lys does not answer, and says to check again', async () => {
    sessionStorage.setItem(KEY, '{"operation":"' + OP + '", "resource": broken');
    await mount('#/requests', asking(refused(503, 'DirectoryUnavailable', 'the directory is closed')));
    await click(button('Check with Lys'));
    expect(text()).toContain('Nothing was cleared');
    expect(sessionStorage.getItem(KEY)).not.toBeNull();
  });
});

describe('Check with Lys, on a decision', () => {
  it('settles a damaged decision by the request no longer waiting', async () => {
    const key = 'lys.pending.decision.' + ADA + '.' + OP;
    sessionStorage.setItem(key, '{broken');
    serve({ ...SERVICE, '/requests': ok({ requests: [asked(OP, 'approved')] }) });
    const container = document.createElement('div'); document.body.append(container);
    const root = createRoot(container);
    await act(async () => { root.render(<ShellProvider><DecisionForm entry={asked(OP) as unknown as AccessRequest} person={ADA} canIssueRoot={false} changed={() => undefined} describe={() => ''} /></ShellProvider>); });
    await click(button('Check with Lys'));
    await settle();
    expect(container.textContent).toContain('Lys has your decision: it went through');
    act(() => root.unmount());
  });

  // Who may decide is worked out from the grants that stand now, so a decider whose own grant ended no longer sees the request.
  it('cannot tell when the request is no longer in the list, and never says the decision did not go through', async () => {
    const key = 'lys.pending.decision.' + ADA + '.' + OP;
    sessionStorage.setItem(key, '{broken');
    serve({ ...SERVICE, '/requests': ok({ requests: [] }) });
    const container = document.createElement('div'); document.body.append(container);
    const root = createRoot(container);
    await act(async () => { root.render(<ShellProvider><DecisionForm entry={asked(OP) as unknown as AccessRequest} person={ADA} canIssueRoot={false} changed={() => undefined} describe={() => ''} /></ShellProvider>); });
    await click(button('Check with Lys'));
    await settle();
    expect(container.textContent).toContain('Lys cannot tell from what you may see whether this went through. Do not decide it again.');
    expect(container.textContent).not.toContain('did not go through');
    expect(button('Forget this kept copy')).not.toBeNull();
    expect(button('Clear it and carry on')).toBeNull();
    act(() => root.unmount());
  });
});

describe('A job change kept unreadably', () => {
  it('asks Lys by a legible operation, and links the job Lys recorded', async () => {
    const JOB = 'op-' + '3'.repeat(32);
    sessionStorage.setItem('lys.pending.role.' + ADA + '.new', '{"operation":"' + JOB + '", broken');
    await mount('#/roles', { ...SERVICE, '/roles': ok({ roles: [] }), ['/operations/' + JOB]: ok({ recorded: true, kind: 'role_change', id: JOB }) });
    await click(button('Create a job'));
    await click(button('Check with Lys'));
    expect(text()).toContain('Lys recorded this: your job change went through');
    expect($('.stuck-record a[href="#/roles/' + JOB + '"]')).not.toBeNull();
  });


  it('cannot be asked about from here, says to look at the list, and clears on her press', async () => {
    sessionStorage.setItem('lys.pending.role.' + ADA + '.new', '{broken');
    await mount('#/roles', { ...SERVICE, '/roles': ok({ roles: [] }) });
    await click(button('Create a job'));
    expect(text()).toContain('Lys cannot tell from what you may see whether this went through. Do not make this change again.');
    expect(($('form[aria-label="Create job"] fieldset') as HTMLFieldSetElement | null)?.disabled).toBe(true);
    expect(button('Clear it and carry on')).toBeNull();
    await click(button('Forget this kept copy'));
    expect(($('form[aria-label="Create job"] fieldset') as HTMLFieldSetElement | null)?.disabled).toBe(false);
  });
});

describe('Check with Lys, on a give', () => {
  it('finds a damaged give Lys already has among the grants, and opens the form on Carry on', async () => {
    const key = 'lys.pending.grant.' + ADA + '.' + ROOT_G;
    const given = GRANTS[0].operation;
    sessionStorage.setItem(key, '{"operation":"' + given + '", "recipient": broken');
    const m = await mount('#/me?tab=account', { ...SERVICE, '/grants/model': ok(AGENT_MODEL), ['/operations/' + given]: ok({ recorded: true, kind: 'grant', id: GRANTS[0].id }) });
    await click($(`[data-act="delegate"][data-g="${ROOT_G}"]`));
    expect($('[data-why-not]')?.textContent).toBe('Check what you gave earlier with Lys first (below).');
    await click(button('Check with Lys'));
    expect(text()).toContain('Lys recorded this: the access you gave went through');
    await click(button('Carry on'));
    expect(sessionStorage.getItem(key)).toBeNull();
    expect(button('Check with Lys')).toBeNull();
    expect(m.posted).toHaveLength(0);
  });
});

/**
 * The hospital line of domain 03, round one: a manager gives access to someone else, the kept copy is damaged, and the
 * check must never send them to a list that hides that person's grant, nor say "did not go through" without a list
 * that would show it. GET /grants holds every grant the caller issued (the server's grant_sight.rs), so it is that list.
 */
describe('Check with Lys, after a give to someone else', () => {
  const CY = 'person-' + 'c'.repeat(32);
  const GIVEN = 'op-' + '8'.repeat(32);
  const PEOPLE: PeopleView = { ...DIRECTORY, people: [...DIRECTORY.people, { id: CY, display_name: 'Cy (test person)', state: 'active', agents: [] }] };
  const passing = GRANTS.map((g) => g.id === ROOT_G ? { ...g, pass_on: { kind: 'to' as const, actions: ['edit', 'view'], recipients: ['person' as const, 'agent' as const] } } : g);
  const cys = { ...GRANTS[2], id: 'grant-' + 'e'.repeat(32), issuer: ADA, holder: CY, responsible: CY, source: ROOT_G, operation: GIVEN };
  const key = 'lys.pending.grant.' + ADA + '.' + ROOT_G;
  const open = async (grants: typeof GRANTS) => {
    const recorded = grants.some((grant) => grant.operation === GIVEN) ? { recorded: true, kind: 'grant', id: cys.id } : { recorded: false };
    const m = await mount('#/me?tab=account', { ...SERVICE, '/grants/model': ok(AGENT_MODEL), '/directory/people': ok(PEOPLE), '/people': ok(PEOPLE), '/grants': ok({ grants, revision: 7 }), ['/operations/' + GIVEN]: ok(recorded) });
    await click($(`[data-act="delegate"][data-g="${ROOT_G}"]`));
    // With no operation left to ask about there is nothing to check: the cannot-tell words stand at once.
    const check = button('Check with Lys');
    if (check) await click(check);
    return m;
  };

  it('finds the give among the access she gave, and sends her to Cy\'s file, where it shows, not to her own list', async () => {
    sessionStorage.setItem(key, '{"operation":"' + GIVEN + '", "recipient":"' + CY + '", broken');
    await open([...passing, cys]);
    expect(text()).toContain('Lys recorded this: the access you gave went through');
    expect($(`a[href="#/file/${CY}/access"]`)?.textContent).toBe('Cy (test person)\u2019s file');
    expect(text()).not.toContain('in your list');
  });

  it('says a legible give Lys has not recorded did not go through, because the access she gave would show it', async () => {
    sessionStorage.setItem(key, '{"operation":"' + GIVEN + '", "recipient":"' + CY + '", broken');
    await open(passing);
    expect(text()).toContain('Lys has no record of the access you gave, so it did not go through.');
    expect(text()).not.toContain('in your list');
  });

  it('cannot tell when no operation is left, says not to give it again, and names Cy\'s file', async () => {
    sessionStorage.setItem(key, '{"recipient":"' + CY + '", broken');
    const m = await open([...passing, cys]);
    expect(text()).toContain('Lys cannot tell from what you may see whether this went through. Do not give it again. Open Cy (test person)\u2019s file, or ask an administrator to look at their access.');
    expect($(`a[href="#/file/${CY}/access"]`)).not.toBeNull();
    expect(text()).not.toContain('did not go through');
    expect(button('Clear it and carry on')).toBeNull();
    await click(button('Forget this kept copy'));
    expect(sessionStorage.getItem(key)).toBeNull();
    expect(m.posted).toHaveLength(0);
  });

  it('cannot tell, and says so without a name, when the recipient cannot be read either', async () => {
    sessionStorage.setItem(key, '{broken');
    await open(passing);
    expect(text()).toContain('Do not give it again. Open the file of the person or agent you gave it to, or ask an administrator to look at their access.');
  });
});
