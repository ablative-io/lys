/**
 * AGENTS-015 R4: the agent file's Give form offers three standing grants, one choice each, above the pickers.
 * No clocks: every wait here is the page changing (a MutationObserver), never a set time.
 */
import { createRoot } from 'react-dom/client';
import type { Root } from 'react-dom/client';
import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import { GiveAccess } from './GiveAccess';
import type { GrantWorld } from '../grants/model';
import type { Entry } from '../people/directory';
import type { DelegateBody, Grant, GrantModel } from '../../generated/grants';
import { ShellProvider } from '../../shell/ShellContext';
import { ADA, BEA, DIRECTORY, GRANTS, ME, MODEL, SCRIBE, ok } from '../../../tests/fixtures';
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
  },
  withheld_from_agents: ['grant'],
};

const held = (id: string, resource: Grant['resource'], actions: string[]): Grant => ({
  ...GRANTS[0], id, holder: ADA, resource, relation: 'owner', actions, source: null,
  pass_on: { kind: 'to', actions, recipients: ['agent'] }, window: { starts_at: 0, ends_at: null }, effective_ends_at: null,
});
const SOURCES: Grant[] = [
  held(PERSON_G, { kind: 'person', id: ADA }, ['agent.create']),
  held(MACHINE_G, { kind: 'machine', id: HOME }, ['machine.agent.set']),
  held(AGENTS_G, { kind: 'person', id: ADA }, ['agent.provisioning.set', 'agent.provisioning.review']),
];

const machine = (id: string, name: string, owner: string) => ({
  id, name, kind: 'laptop', runtime: 'runner', team: null, slots: 2, may_run: [], may_reach: [], named_by: owner, named_at: 1,
  state: 'in_use', retired_at: null, last_report_at: null,
});

const scribe: Entry = { id: SCRIBE, display_name: 'Scribe', state: 'active', kind: 'agent', role: null,
  person: { id: ADA, display_name: 'Ada (test person)', state: 'active' } };
const notMine: Entry = { id: OTHER_AGENT, display_name: 'Lantern', state: 'active', kind: 'agent', role: null,
  person: { id: BEA, display_name: 'Bea (test person)', state: 'active' } };

const CREATE = 'Lets Scribe register agents that answer to you; it then may set up, start, stop and restart each agent it creates.';
const RUN = 'Lets Scribe allow your agents to run on Workshop.';
const PROFILES = 'Lets Scribe set up and review your agents’ profiles; a version Scribe sets itself is reviewed by someone else.';
const WHY = 'Six approvals for every agent is security too awkward to use';

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

async function opened(x: Entry, machines = [machine(HOME, 'Workshop', ADA), machine(AWAY, 'Bea’s laptop', BEA)]) {
  const posted: { path: string; body: unknown }[] = [];
  let made = 0;
  serve({
    '/network': ok({ machines, reports_served: true }),
    'POST /grants': (body) => {
      made += 1;
      return ok({ operation: (body as DelegateBody).operation, grant: 'grant-' + hex(100 + made), index: made, receipt: { caller: ADA } });
    },
  }, posted);
  const world: GrantWorld = { me: ME, people: DIRECTORY, list: { grants: SOURCES, revision: 7 }, model,
    byId: new Map(SOURCES.map((g) => [g.id, g])), who: new Map([
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

/** Press one standing choice and wait for its outcome line. */
async function choose(container: HTMLElement, act: string) {
  const button = container.querySelector<HTMLButtonElement>(`[data-act="${act}"]`);
  expect(button, act).not.toBeNull();
  button?.click();
  await until(container, () => container.querySelector('[data-standing-receipt], [data-standing-refused]') !== null);
}

const actionsOf = (body: unknown): string[] => model.relations[(body as DelegateBody).relation] ?? [];

describe('The standing grants on an agent’s Give form', () => {
  it('offers the three choices with their sentences, once saying why each is one choice, above the pickers', async () => {
    const { container } = await opened(scribe);
    const text = container.textContent ?? '';
    for (const words of ['Create agents for me', 'Run my agents on this computer', 'Set up and review this kind of profile', CREATE, RUN, PROFILES]) {
      expect(text).toContain(words);
    }
    expect(text.split(WHY).length - 1).toBe(1);
    expect(text).toContain('the standing grant is your consent, and every act under it is on the record as Scribe’s');
    const standing = container.querySelector('[data-standing]');
    const picker = container.querySelector('select[aria-label="Grant to give"]');
    expect(standing && picker && standing.compareDocumentPosition(picker) & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy();
    // Only the person's own computer is offered by name.
    const computers = [...container.querySelectorAll<HTMLOptionElement>('select[aria-label="Computer to give"] option')].map((o) => o.textContent);
    expect(computers).toEqual(['Workshop']);
  });

  it('Create agents for me posts one grant of exactly agent.create on the person, and the receipt repeats the sentence', async () => {
    const { container, posted } = await opened(scribe);
    await choose(container, 'standing-create');
    expect(posted.map((p) => p.path)).toEqual(['/grants']);
    expect(actionsOf(posted[0].body)).toEqual(['agent.create']);
    expect(posted[0].body).toMatchObject({ source: PERSON_G, recipient: SCRIBE, responsible: ADA, resource: { kind: 'person', id: ADA },
      relation: 'only.agent.create', pass_on: { kind: 'use_only' }, route: 'browser' });
    expect(container.querySelector('[data-standing-receipt]')?.textContent).toContain(CREATE);
  });

  it('Run my agents on this computer posts one grant of exactly machine.agent.set on the chosen own computer', async () => {
    const { container, posted } = await opened(scribe);
    await choose(container, 'standing-machine');
    expect(posted.map((p) => p.path)).toEqual(['/grants']);
    expect(actionsOf(posted[0].body)).toEqual(['machine.agent.set']);
    expect(posted[0].body).toMatchObject({ source: MACHINE_G, recipient: SCRIBE, responsible: ADA, pass_on: { kind: 'use_only' }, resource: { kind: 'machine', id: HOME }, relation: 'only.machine.agent.set' });
    expect(container.querySelector('[data-standing-receipt]')?.textContent).toContain(RUN);
  });

  it('Set up and review this kind of profile posts exactly agent.provisioning.set and agent.provisioning.review, one grant each', async () => {
    const { container, posted } = await opened(scribe);
    await choose(container, 'standing-profiles');
    expect(posted.map((p) => p.path)).toEqual(['/grants', '/grants']);
    expect(posted.flatMap((p) => actionsOf(p.body))).toEqual(['agent.provisioning.set', 'agent.provisioning.review']);
    for (const p of posted) expect(p.body).toMatchObject({ source: AGENTS_G, recipient: SCRIBE, responsible: ADA, resource: { kind: 'person', id: ADA }, pass_on: { kind: 'use_only' } });
    expect(posted.map((p) => (p.body as DelegateBody).relation)).toEqual(['only.agent.provisioning.set', 'only.agent.provisioning.review']);
    expect(container.querySelector('[data-standing-receipt]')?.textContent).toContain(PROFILES);
  });

  it('with no computer of the person’s own, the second choice is absent and the words say so', async () => {
    const { container } = await opened(scribe, [machine(AWAY, 'Bea’s laptop', BEA)]);
    expect(container.querySelector('[data-act="standing-machine"]')).toBeNull();
    expect(container.querySelector('select[aria-label="Computer to give"]')).toBeNull();
    expect(container.textContent).toContain('You have no computer of your own to give');
    expect(container.querySelector('[data-act="standing-create"]')).not.toBeNull();
    expect(container.querySelector('[data-act="standing-profiles"]')).not.toBeNull();
  });

  it('for an agent the person does not answer for, none of the three renders', async () => {
    const { container, posted } = await opened(notMine);
    expect(container.querySelector('[data-standing]')).toBeNull();
    for (const words of ['Create agents for me', 'Run my agents on this computer', 'Set up and review this kind of profile', WHY]) {
      expect(container.textContent).not.toContain(words);
    }
    expect(posted).toEqual([]);
  });
});
