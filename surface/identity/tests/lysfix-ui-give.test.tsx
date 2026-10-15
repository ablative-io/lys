import { act } from 'react';
import { createRoot, type Root } from 'react-dom/client';
import { afterEach, beforeEach, expect, it, vi } from 'vitest';
import { TabBody } from '../src/features/file/sections';
import type { FileData } from '../src/features/file/IdentityFile';
import type { GrantWorld } from '../src/features/grants/model';
import { entries } from '../src/features/people/directory';
import type { DelegateBody, Grant } from '../src/generated/grants';
import { ShellProvider } from '../src/shell/ShellContext';
import { ACCESS } from '../src/shell/help/concepts/access';
import { START } from '../src/shell/help/concepts/start';
import { TROUBLE } from '../src/shell/help/concepts/trouble';
import { PAGES } from '../src/shell/help/pages';
import { ADA, DIRECTORY, GRANTS, LEDGER_G, ME, MODEL, SCRIBE, SCRIBE_G, SCRIBE_VIEW, ok } from './fixtures';
import { serve } from './harness';

const kind = 'cambium.channel';
const first: Grant = { ...GRANTS[0], resource: { kind, id: 'general' }, relation: 'reader', actions: ['read'], pass_on: { kind: 'to', actions: ['read'], recipients: ['agent'] } };
const second: Grant = { ...first, id: LEDGER_G, resource: { kind, id: 'ward' } };
const roots: Root[] = [];

beforeEach(() => sessionStorage.clear());
afterEach(() => {
  for (const root of roots.splice(0)) act(() => root.unmount());
});

async function open() {
  const posted: { path: string; body: unknown }[] = [];
  const done = vi.fn();
  serve(
    {
      '/roles': ok({ roles: [] }),
      [`/grants/cannot-give?route=browser&source=${first.id}&recipient=${SCRIBE}`]: ok({ source: first.id, recipient: SCRIBE, items: [] }),
      [`/grants/cannot-give?route=browser&source=${second.id}&recipient=${SCRIBE}`]: ok({ source: second.id, recipient: SCRIBE, items: [] }),
      'POST /grants': (body) => ok({ operation: (body as DelegateBody).operation, grant: SCRIBE_G, receipt: { caller: ADA } }),
    },
    posted
  );
  const grants: GrantWorld = {
    me: ME,
    people: DIRECTORY,
    list: { grants: [first, second], revision: 7 },
    model: { ...MODEL, kinds: { [kind]: { agents: ['read'], relations: { reader: ['read'] } } } },
    byId: new Map([first, second].map((grant) => [grant.id, grant])),
    who: new Map([
      [ADA, { name: 'Ada', state: 'active', kind: 'person', responsible: null }],
      [SCRIBE, { name: 'Scribe', state: 'active', kind: 'agent', responsible: ADA }],
    ]),
  };
  const x = entries(DIRECTORY).find((entry) => entry.id === SCRIBE);
  if (!x) throw new Error('Scribe was not in the directory');
  const data: FileData = { x, agent: SCRIBE_VIEW, agents: [], receipts: [], grants };
  const page = document.createElement('div');
  document.body.appendChild(page);
  const root = createRoot(page);
  roots.push(root);
  await act(async () => {
    root.render(
      <ShellProvider>
        <TabBody tab="access" data={data} reload={done} />
      </ShellProvider>
    );
  });
  const give = page.querySelector<HTMLButtonElement>('[data-act="grant"]');
  if (!give) throw new Error('Give access was not shown');
  await act(async () => {
    give.click();
  });
  return { page, posted, done };
}

async function choose(select: HTMLSelectElement, value: string) {
  await act(async () => {
    select.value = value;
    select.dispatchEvent(new Event('change', { bubbles: true }));
  });
}

it('starts with no grant chosen, offers every passable grant by resource and submits the second grant', async () => {
  const { page, posted, done } = await open();
  const picker = page.querySelector<HTMLSelectElement>('[aria-label="Grant to give"]');
  expect(picker).not.toBeNull();
  if (!picker) throw new Error('Grant choices were not shown');
  expect(picker.value).toBe('');
  expect([...picker.options].map((option) => option.value)).toEqual(['', first.id, second.id]);
  expect(picker.options[1]?.textContent).toContain('cambium.channel:general');
  expect(picker.options[2]?.textContent).toContain('cambium.channel:ward');
  expect(page.querySelector('[data-act="delegatedo"]')).toBeNull();
  await choose(picker, second.id);
  const action = page.querySelector<HTMLInputElement>('input[name="action"][value="read"]');
  if (!action) throw new Error('The chosen grant had no action choice');
  await act(async () => {
    action.click();
  });
  const give = page.querySelector<HTMLButtonElement>('[data-act="delegatedo"]');
  if (!give) throw new Error('The chosen grant had no Give action');
  await act(async () => {
    give.click();
  });
  expect(posted).toEqual([{ path: '/grants', body: expect.objectContaining({ source: second.id, resource: second.resource, recipient: SCRIBE }) }]);
  expect(done).toHaveBeenCalledOnce();
});

it('resets action choices when the person chooses another source grant', async () => {
  const { page, posted } = await open();
  const picker = page.querySelector<HTMLSelectElement>('[aria-label="Grant to give"]');
  expect(picker).not.toBeNull();
  if (!picker) throw new Error('Grant choices were not shown');
  await choose(picker, first.id);
  const action = page.querySelector<HTMLInputElement>('input[name="action"][value="read"]');
  if (!action) throw new Error('The chosen grant had no action choice');
  await act(async () => {
    action.click();
  });
  await choose(picker, second.id);
  expect(page.querySelector<HTMLInputElement>('input[name="action"][value="read"]')?.checked).toBe(false);
  expect(page.querySelector<HTMLButtonElement>('[data-act="delegatedo"]')?.disabled).toBe(true);
  expect(posted).toEqual([]);
});

it.each([
  ['access concept', ACCESS.find((entry) => entry.id === 'access.give')?.body.find((words) => words.includes('Let me give agents access'))],
  ['start concept', START.find((entry) => entry.id === 'start.channels')?.body.find((words) => words.includes('Let me give agents access'))],
  ['trouble concept', TROUBLE.find((entry) => entry.id === 'trouble.no-access-to-give')?.body.find((words) => words.includes('Let me give agents access'))],
  ['page help', PAGES['file:access'].regions.find((region) => region.sel === '[data-act="agent-roots"]')?.text],
])('limits the agent-roots button to directory access in %s', (_label, words) => {
  expect(words).toContain('agents and apps directories only');
  expect(words).toContain('Cambium');
});
