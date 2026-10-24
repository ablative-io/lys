/**
 * AGENTS-017 R4: the agent file's Give form offers one standing choice, above the pickers: let the agent set up and
 * run agents on a computer of the person's. Give posts five grants in order (the fifth, issuing the person's agents'
 * certificates, since 24 October 2026); an administrator who holds nothing to give one from has Lys record their roots
 * first. Below it, whoever holds organisation.grant gives the agent standing in the organisation itself. No clocks: every wait here is the page changing (a
 * MutationObserver), never a set time.
 */
import { createRoot } from 'react-dom/client';
import type { Root } from 'react-dom/client';
import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import { GiveAccess } from './GiveAccess';
import { grantNo, type GrantWorld } from '../grants/model';
import type { Entry } from '../people/directory';
import type { DelegateBody, Grant, GrantModel } from '../../generated/grants';
import { ShellProvider } from '../../shell/ShellContext';
import { ADA, BEA, DIRECTORY, FOUNDING_G, GRANTS, ME, MODEL, ORGANISATION, OWN, SCRIBE, ok, refused } from '../../../tests/fixtures';
import type { Route } from '../../../tests/fixtures';
import type { PeopleView } from '../../generated';
import { serve } from '../../../tests/harness';

const hex = (n: number) => n.toString(16).padStart(32, '0');
const PERSON_G = 'grant-' + hex(71);
const MACHINE_G = 'grant-' + hex(72);
const AGENTS_G = 'grant-' + hex(73);
const HOME = 'op-' + hex(81);
const AWAY = 'op-' + hex(82);
const OTHER_AGENT = 'agent-' + hex(91);

const model: GrantModel = {
  ...MODEL,
  relations: {
    ...MODEL.relations,
    'only.agent.create': ['agent.create'],
    'only.machine.agent.set': ['machine.agent.set'],
    'only.agent.provisioning.set': ['agent.provisioning.set'],
    'only.agent.provisioning.review': ['agent.provisioning.review'],
    'only.agent.certificate.issue': ['agent.certificate.issue'],
  },
  withheld_from_agents: ['grant'],
};
/** The same model as a founded organisation's service serves it: the administrator relation, with its sentences. */
const WITH_ORGANISATION: GrantModel = { ...model,
  relations: { ...model.relations, administrator: ['organisation.administer', 'organisation.admit', 'organisation.grant', 'organisation.read'] },
  action_sentences: { ...model.action_sentences, 'organisation.administer': 'Administer this organisation', 'organisation.admit': 'Add people to this organisation',
    'organisation.grant': 'Give access directly in this organisation', 'organisation.read': 'Read this organisation' } };
const FOUNDING = GRANTS.find((grant) => grant.id === FOUNDING_G) as Grant;

const held = (id: string, resource: Grant['resource'], actions: string[]): Grant => ({
  ...GRANTS[0], id, holder: ADA, resource, relation: 'owner', actions, source: null,
  pass_on: { kind: 'to', actions, recipients: ['agent'] }, window: { starts_at: 0, ends_at: null }, effective_ends_at: null,
});
const SOURCES: Grant[] = [
  held(PERSON_G, { kind: 'person', id: ADA }, ['agent.create']),
  held(MACHINE_G, { kind: 'machine', id: HOME }, ['machine.agent.set']),
  held(AGENTS_G, { kind: 'person', id: ADA }, ['agent.provisioning.set', 'agent.provisioning.review', 'agent.certificate.issue']),
];

const machine = (id: string, name: string, owner: string, runtime = 'runner') => ({
  id, name, kind: 'laptop', runtime, team: null, slots: 2, may_run: [], may_reach: [], named_by: owner, named_at: 1,
  state: 'in_use', retired_at: null, last_report_at: null,
});

const scribe: Entry = { id: SCRIBE, display_name: 'Scribe', state: 'active', kind: 'agent', role: null,
  person: { id: ADA, display_name: 'Ada (test person)', state: 'active' } };
const notMine: Entry = { id: OTHER_AGENT, display_name: 'Lantern', state: 'active', kind: 'agent', role: null,
  person: { id: BEA, display_name: 'Bea (test person)', state: 'active' } };

const TITLE = 'Let Scribe set up and run agents on Workshop';
const CREATE = 'Lets Scribe register agents that answer to you; it then may set up, start, stop and restart each agent it creates.';
const RUN = 'Lets Scribe allow your agents to run on Workshop.';
const PROFILES = 'Lets Scribe set up and review your agents’ profiles; a version Scribe sets itself is reviewed by someone else.';
const CERTIFICATES = 'Lets Scribe issue your agents’ certificates, the keys they act with, so a new or re-keyed agent needs no sign-in of yours.';
const WHY = 'Six approvals for every agent is security too awkward to use';
const ORDER = ['agent.create', 'machine.agent.set', 'agent.provisioning.set', 'agent.provisioning.review', 'agent.certificate.issue'];
const OLD_CHOICES = ['Create agents for me', 'Run my agents on this computer', 'Set up and review this kind of profile'];

let root: Root | null = null;
let actEnvironment: boolean | undefined;
beforeEach(() => {
  sessionStorage.clear();
  // The page settles by its own events here, observed below; React schedules as it does in a browser.
  const scope = globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean };
  actEnvironment = scope.IS_REACT_ACT_ENVIRONMENT;
  scope.IS_REACT_ACT_ENVIRONMENT = false;
});
afterEach(() => {
  root?.unmount();
  root = null;
  (globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = actEnvironment;
});

/** Resolves when `ready` holds of the page: checked now, then on every change to it. */
function until(container: HTMLElement, ready: () => boolean): Promise<void> {
  return new Promise((resolve) => {
    if (ready()) { resolve(); return; }
    const watch = new MutationObserver(() => { if (ready()) { watch.disconnect(); resolve(); } });
    watch.observe(container, { childList: true, subtree: true, characterData: true, attributes: true });
  });
}

interface Opening {
  machines?: ReturnType<typeof machine>[];
  people?: PeopleView;
  grants?: Grant[];
  /** The answer to the n-th POST /grants (1-based), when not the plain confirmation. */
  answer?: (made: number, body: DelegateBody) => ReturnType<typeof ok> | null;
  routes?: Record<string, Route>;
  model?: GrantModel;
}

async function opened(x: Entry, given: Opening = {}) {
  const machines = given.machines ?? [machine(HOME, 'Workshop', ADA), machine(AWAY, 'Bea’s laptop', BEA)];
  const grants = given.grants ?? SOURCES;
  const posted: { path: string; body: unknown }[] = [];
  let made = 0;
  serve({
    '/network': ok({ machines, reports_served: true }),
    'POST /grants': (body) => {
      made += 1;
      const delegated = body as DelegateBody;
      return given.answer?.(made, delegated)
        ?? ok({ operation: delegated.operation, grant: 'grant-' + hex(100 + made), index: made, receipt: { caller: ADA } });
    },
    ...given.routes,
  }, posted);
  const world: GrantWorld = { me: ME, people: given.people ?? DIRECTORY, list: { grants, revision: 7 }, model: given.model ?? model,
    byId: new Map(grants.map((g) => [g.id, g])), who: new Map([
      [ADA, { name: 'Ada', state: 'active', kind: 'person', responsible: null }],
      [SCRIBE, { name: 'Scribe', state: 'active', kind: 'agent', responsible: ADA }],
      [OTHER_AGENT, { name: 'Lantern', state: 'active', kind: 'agent', responsible: BEA }],
    ]) };
  const container = document.createElement('div');
  document.body.appendChild(container);
  root = createRoot(container);
  root.render(<ShellProvider><GiveAccess x={x} w={world} query="" reload={() => undefined} /></ShellProvider>);
  await until(container, () => container.querySelector('[data-act="grant"]') !== null);
  container.querySelector<HTMLButtonElement>('[data-act="grant"]')?.click();
  // The Give panel for the person's own agent; for anyone else's, the toast that says who may give.
  await until(container, () => container.querySelector('section[aria-label="Give"]') !== null || x.person?.id !== ADA);
  // The computers are read once the panel opens; the page says it is reading until the answer is drawn.
  await until(container, () => container.querySelector('[data-standing-reading]') === null);
  return { container, posted };
}

/** Press the standing choice and wait for its outcome line. */
async function give(container: HTMLElement) {
  const button = container.querySelector<HTMLButtonElement>('[data-act="standing-all"]');
  expect(button).not.toBeNull();
  button?.click();
  await until(container, () => container.querySelector('[data-standing-receipt], [data-standing-refused]') !== null);
}

const actionsOf = (body: unknown): string[] => model.relations[(body as DelegateBody).relation] ?? [];
const grantsPosted = (posted: { path: string; body: unknown }[]) => posted.filter((p) => p.path === '/grants');

describe('The one standing choice on an agent’s Give form', () => {
  it('offers one choice naming the computer, with the four sentences, once saying why it is one choice, above the pickers', async () => {
    const { container } = await opened(scribe);
    const choices = container.querySelectorAll('[data-standing-choice]');
    expect(choices.length).toBe(1);
    const text = container.textContent ?? '';
    for (const words of [TITLE, CREATE, RUN, PROFILES, CERTIFICATES]) expect(text).toContain(words);
    for (const words of OLD_CHOICES) expect(text).not.toContain(words);
    expect(text.split(WHY).length - 1).toBe(1);
    expect(text).toContain('the standing grant is your consent, and every act under it is on the record as Scribe’s');
    const standing = container.querySelector('[data-standing]');
    const picker = container.querySelector('select[aria-label="Grant to give"]');
    expect(standing && picker && standing.compareDocumentPosition(picker) & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy();
    // One computer of the person's own: named in the title, no list to choose from.
    expect(container.querySelector('select[aria-label="Computer to give"]')).toBeNull();
  });

  it('with more than one computer of their own, the one Lys runs on is chosen first and the others offered', async () => {
    const { container } = await opened(scribe, { machines: [machine(AWAY, 'Garage', ADA), machine(HOME, 'Workshop', ADA, 'lys-runner')] });
    const select = container.querySelector<HTMLSelectElement>('select[aria-label="Computer to give"]');
    expect([...(select?.options ?? [])].map((o) => o.textContent)).toEqual(['Garage', 'Workshop']);
    expect(select?.value).toBe(HOME);
    expect(container.textContent).toContain(TITLE);
  });

  it('Give posts the five grants in order, each exactly one act, use only, and one receipt names all five', async () => {
    const { container, posted } = await opened(scribe);
    await give(container);
    expect(posted.map((p) => p.path)).toEqual(['/grants', '/grants', '/grants', '/grants', '/grants']);
    expect(posted.map((p) => actionsOf(p.body))).toEqual(ORDER.map((action) => [action]));
    expect(posted.map((p) => (p.body as DelegateBody).source)).toEqual([PERSON_G, MACHINE_G, AGENTS_G, AGENTS_G, AGENTS_G]);
    expect(posted.map((p) => (p.body as DelegateBody).resource)).toEqual([
      { kind: 'person', id: ADA }, { kind: 'machine', id: HOME }, { kind: 'person', id: ADA }, { kind: 'person', id: ADA }, { kind: 'person', id: ADA }]);
    for (const p of posted) {
      expect(p.body).toMatchObject({ recipient: SCRIBE, responsible: ADA, pass_on: { kind: 'use_only' }, route: 'browser' });
    }
    const receipt = container.querySelectorAll('[data-standing-receipt]');
    expect(receipt.length).toBe(1);
    const words = receipt[0].textContent ?? '';
    for (const part of [TITLE, CREATE, RUN, PROFILES, CERTIFICATES, ...ORDER]) expect(words).toContain(part);
    for (const made of [1, 2, 3, 4, 5]) expect(words).toContain(grantNo('grant-' + hex(100 + made)));
    expect(container.querySelector('[data-standing-refused]')).toBeNull();
  });

  it('an administrator holding nothing to give has Lys record their roots first, reads the grants again, then gives the five', async () => {
    const { container, posted } = await opened(scribe, {
      grants: [],
      routes: {
        'POST /grants/agent-roots': ok({ operation: 'op-' + hex(5), grants: [PERSON_G, MACHINE_G, AGENTS_G] }),
        '/grants': ok({ grants: SOURCES, revision: 8 }),
      },
    });
    // Nothing passable yet, so the pickers say so; the standing choice is still offered to the administrator.
    expect(container.querySelector('[data-act="standing-all"]')).not.toBeNull();
    await give(container);
    expect(posted.map((p) => p.path)).toEqual(['/grants/agent-roots', '/grants', '/grants', '/grants', '/grants', '/grants']);
    expect(grantsPosted(posted).map((p) => actionsOf(p.body))).toEqual(ORDER.map((action) => [action]));
    expect(container.querySelector('[data-standing-receipt]')?.textContent).toContain(TITLE);
  });

  it('a refusal on the third grant names that step and the two given before it, and sends nothing after', async () => {
    const { container, posted } = await opened(scribe, {
      answer: (made) => (made === 3 ? refused(403, 'NotPermitted', 'the set act is withheld today') : null),
    });
    await give(container);
    expect(grantsPosted(posted).map((p) => actionsOf(p.body))).toEqual([['agent.create'], ['machine.agent.set'], ['agent.provisioning.set']]);
    const alert = container.querySelector('[data-standing-refused]')?.textContent ?? '';
    expect(alert).toContain('Lys refused agent.provisioning.set: the set act is withheld today');
    expect(alert).toContain('Given before it: agent.create (' + grantNo('grant-' + hex(101)) + '), machine.agent.set (' + grantNo('grant-' + hex(102)) + ')');
    expect(alert).not.toContain('agent.provisioning.review');
    expect(container.querySelector('[data-standing-receipt]')).toBeNull();
  });

  it('a person who is not the administrator never has roots recorded: a missing source is said, and nothing is posted', async () => {
    const { container, posted } = await opened(scribe, { people: OWN, grants: [SOURCES[0]], machines: [machine(AWAY, 'Bea’s laptop', BEA)] });
    expect(container.querySelector('[data-act="standing-all"]')).toBeNull();
    expect(container.textContent).toContain('machine.agent.set: You have no computer of your own to give.');
    expect(posted).toEqual([]);
  });

  it('for an agent the person does not answer for, the standing choice does not render', async () => {
    const { container, posted } = await opened(notMine);
    expect(container.querySelector('[data-standing]')).toBeNull();
    for (const words of [TITLE, WHY]) expect(container.textContent).not.toContain(words);
    expect(posted).toEqual([]);
  });
});

describe('Standing in the organisation on an agent’s Give form (24 October 2026)', () => {
  const standing = (container: HTMLElement) => container.querySelector<HTMLSelectElement>('select[aria-label="Standing in the organisation"]');

  it('offers the organisation’s standing relations by sentence to the holder of organisation.grant, and gives administrator as administrator standing', async () => {
    const { container, posted } = await opened(scribe, { grants: [...SOURCES, FOUNDING], model: WITH_ORGANISATION });
    const block = container.querySelector('[data-organisation-standing]');
    expect(block?.textContent).toContain('In Example organisation');
    expect([...(standing(container)?.options ?? [])].map((o) => o.value)).toEqual(['', 'administrator']);
    expect(standing(container)?.textContent).toContain('Add people to this organisation');
    expect(standing(container)?.textContent).not.toContain('Edit this resource');
    const button = container.querySelector<HTMLButtonElement>('[data-act="organisation-standing"]');
    expect(button?.disabled).toBe(true);
    const select = standing(container);
    if (!select) throw new Error('no standing select');
    const setter = Object.getOwnPropertyDescriptor(HTMLSelectElement.prototype, 'value')?.set;
    setter?.call(select, 'administrator');
    select.dispatchEvent(new Event('change', { bubbles: true }));
    await until(container, () => container.querySelector<HTMLButtonElement>('[data-act="organisation-standing"]')?.disabled === false);
    container.querySelector<HTMLButtonElement>('[data-act="organisation-standing"]')?.click();
    await until(container, () => container.querySelector('[data-organisation-receipt], [data-organisation-refused]') !== null);
    expect(grantsPosted(posted)).toHaveLength(1);
    expect(grantsPosted(posted)[0].body).toMatchObject({
      source: 'administrator', recipient: SCRIBE, responsible: ADA, relation: 'administrator', mode: 'outright', route: 'browser',
      resource: { kind: 'organisation', id: ORGANISATION }, pass_on: { kind: 'use_only' }, window: { ends_at: null },
    });
    const words = container.querySelector('[data-organisation-receipt]')?.textContent ?? '';
    expect(words).toContain('Scribe is now administrator of Example organisation (' + grantNo('grant-' + hex(101)) + ')');
    expect(words).toContain('Add people to this organisation');
  });

  it('is not drawn for a person holding no organisation.grant, nor for an agent the person does not answer for', async () => {
    const mine = await opened(scribe, { model: WITH_ORGANISATION });
    expect(mine.container.querySelector('[data-organisation-standing]')).toBeNull();
    root?.unmount();
    const theirs = await opened(notMine, { grants: [...SOURCES, FOUNDING], model: WITH_ORGANISATION });
    expect(theirs.container.querySelector('[data-organisation-standing]')).toBeNull();
  });
});
