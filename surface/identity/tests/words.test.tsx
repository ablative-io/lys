/** AGENTS-001 R5: the Usage screen shows and sets words, variables and schedules with named refusals: an agent-layer warning is saved from the revision read, a preview renders a slot and sends nothing, a variable is set from the map's revision, a schedule is set on the agent and its fired occurrence shows in the table, and a stale save is shown by its name. */
import { describe, expect, it } from 'vitest';
import { $, click, mount, text, type, unmountAll } from './harness';
import { SCRIBE, SERVICE, ok, refused } from './fixtures';
import type { Route } from './fixtures';
import type { ScheduleItem, VariablesRead, WordsForAgent } from '../src/features/usage/contract';
import { budgetsView } from './budget-fixtures';

const file = '#/file/' + SCRIBE + '/budgets';
const words = '/agents/' + SCRIBE + '/words';
const variables = '/agents/' + SCRIBE + '/variables';
const holder = { kind: 'agent' as const, id: SCRIBE };

const resolved = (text: string, source: string): WordsForAgent['resolved'][number] => ({ slot: 'context_warning', text, source, contributed: [] });

function forAgent(held: WordsForAgent['held'] = {}): WordsForAgent {
  const slots = ['context_warning', 'preparation', 'compaction', 'wake_up', 'scheduled_reminder'] as const;
  return {
    agent: SCRIBE,
    resolved: slots.map((slot) => ({ ...resolved(slot === 'compaction' ? '' : '[Lys context watch] built in {{context_percent}}', slot === 'compaction' ? 'profile' : 'built_in'), slot, text: slot === 'compaction' ? null : '[Lys context watch] built in {{context_percent}}' })),
    held,
  };
}

const fired: ScheduleItem = {
  schedule: { id: 'op-sched-1', at: 1800000000, interval: 3600, max_occurrences: 2, recipients: [holder], source: { kind: 'text', text: 'Stand-up.' }, author: 'person-1', set_at: 1790000000 },
  fired: [{ operation: 'op-f1', schedule: 'op-sched-1', occurrence: 1, due: 1800000000, fired: 1800000001, coalesced: 1, sent: [{ recipient: holder, session: 'session-1', operation: 'op-d1', text: 'Stand-up.', contributed: [], missing: [], state: 'delivered', words: 'typed', at: 1800000001 }] }],
  next_due: 1800003600,
};

/** A service that keeps the words, variables and schedules it is given. */
function keeping(stale = false): Record<string, Route> {
  let held: WordsForAgent['held'] = {};
  let map: VariablesRead = { scope: holder, revision: 0, values: {}, expired: [] };
  const schedules: ScheduleItem[] = [fired];
  return {
    ...SERVICE,
    ['/budgets/agent/' + SCRIBE]: ok(budgetsView(holder)),
    ['/agents/' + SCRIBE + '/usage']: ok({ agent: SCRIBE, used: [], receipts: [], last_reported_ms: 1790000000000 }),
    ['/agents/' + SCRIBE + '/goals']: ok({ goals: [] }),
    [words]: () => ok(forAgent(held)),
    ['POST ' + words + '/context_warning']: (body) => {
      const given = body as { setting: { kind: string; text?: string }; revision: number };
      if (stale) return refused(409, 'words_stale', 'agent/context_warning is at revision 1, not 0');
      expect(given.revision).toBe(0);
      held = { context_warning: { setting: given.setting as WordsForAgent['held'][string]['setting'], revision: 1, by: 'person-1', at: 1 } };
      return ok({ revision: 1 });
    },
    ['POST /words/preview']: (body) => {
      const given = body as { slot: string; agent: string; numbers: Record<string, string> };
      expect(given.agent).toBe(SCRIBE);
      return ok({ slot: given.slot, text: '[Lys context watch] rendered at ' + given.numbers.context_percent, source: 'built_in', contributed: [], missing: [] });
    },
    [variables]: () => ok(map),
    ['POST ' + variables]: (body) => {
      const given = body as { revision: number; values: Record<string, unknown> };
      expect(given.revision).toBe(map.revision);
      const values = { ...map.values };
      for (const [name, value] of Object.entries(given.values)) {
        if (value === null) delete values[name];
        else values[name] = { value, revision: map.revision + 1, author: 'person-1', at: 2 };
      }
      map = { ...map, revision: map.revision + 1, values };
      return ok(map);
    },
    '/schedules': () => ok({ schedules }),
    'POST /schedules': (body) => {
      const given = body as { operation: string; at: number; recipients: unknown[]; source: { kind: string; text: string }; interval?: number };
      expect(given.recipients).toEqual([holder]);
      const item: ScheduleItem = { schedule: { id: given.operation, at: given.at, interval: given.interval, recipients: [holder], source: { kind: 'text', text: given.source.text }, author: 'person-1', set_at: 3 }, fired: [], next_due: given.at };
      schedules.push(item);
      return ok(item);
    },
  };
}

async function submit(form: string): Promise<void> {
  const found = $('form[aria-label="' + form + '"]');
  await click(found?.querySelector('button[type="submit"]') ?? $('button[type="submit"][form="' + found?.id + '"]'));
}

describe('Words, variables and schedules on the agent file', () => {
  it('shows the five slots resolved, saves an agent-layer warning from revision 0, and previews it', async () => {
    const mounted = await mount(file, keeping());
    expect($('section[aria-label="Words"]')).not.toBeNull();
    const heads = [...document.querySelectorAll('section[aria-label="Words"] thead th')].map((th) => th.textContent);
    expect(heads).toEqual(['Slot', 'What the agent is sent', 'From', "This agent's layer"]);
    expect(document.querySelectorAll('section[aria-label="Words"] tbody tr')).toHaveLength(5);
    expect(text()).toContain('Not set: the profile names the command.');

    const form = $('form[aria-label="Set the context warning words"]');
    const kind = form?.querySelector('select');
    if (!kind) throw new Error('no kind select');
    await (await import('./harness')).choose(kind, 'text');
    await type(form?.querySelector('input[name="text"]') ?? null, '[Lys context watch] Scribe, at {{context_percent}} percent.');
    await submit('Set the context warning words');
    const saved = mounted.posted.find((post) => post.path === words + '/context_warning');
    expect(saved?.body).toEqual({ setting: { kind: 'text', text: '[Lys context watch] Scribe, at {{context_percent}} percent.' }, revision: 0 });
    expect(text()).toContain('Context warning words saved.');

    await click($('button[aria-label="Preview the context_warning words"]'));
    expect($('[aria-label="Preview of context_warning"]')?.textContent).toBe('[Lys context watch] rendered at 50');
    expect(mounted.posted.filter((post) => post.path === '/words/preview')).toHaveLength(1);
    unmountAll();
  });

  it('shows a stale save by its name and keeps the form', async () => {
    await mount(file, keeping(true));
    const form = $('form[aria-label="Set the context warning words"]');
    await (await import('./harness')).choose(form?.querySelector('select') ?? null, 'text');
    await type(form?.querySelector('input[name="text"]') ?? null, 'Late words.');
    await submit('Set the context warning words');
    expect(text()).toContain('words_stale');
    expect(text()).toContain('revision 1, not 0');
    unmountAll();
  });

  it('sets a variable from the revision read and removes one', async () => {
    const mounted = await mount(file, keeping());
    expect($('section[aria-label="Variables"]')).not.toBeNull();
    expect(text()).toContain('No variable is set for this agent.');
    await type($('input[aria-label="Variable name"]'), 'focus');
    await type($('input[aria-label="Variable value"]'), 'the door install');
    await submit('Set a variable');
    expect(mounted.posted.find((post) => post.path === variables)?.body).toEqual({ revision: 0, values: { focus: 'the door install' } });
    expect(text()).toContain('Variable set.');
    expect($('section[aria-label="Variables"] tbody')?.textContent).toContain('the door install');
    await click($('button[aria-label="Remove the variable focus"]'));
    const removed = mounted.posted.filter((post) => post.path === variables).at(-1);
    expect(removed?.body).toEqual({ revision: 1, values: { focus: null } });
    unmountAll();
  });

  it('shows a schedule with its fired occurrence and sets a new one on the agent', async () => {
    const mounted = await mount(file, keeping());
    const section = $('section[aria-label="Schedules"]');
    expect(section).not.toBeNull();
    expect(section?.textContent).toContain('Stand-up.');
    expect(section?.textContent).toContain('1: due');
    expect(section?.textContent).toContain('delivered');
    await type($('input[aria-label="Schedule text"]'), 'Read the room.');
    await type($('input[aria-label="First instant"]'), '2099-01-01T09:00');
    await type($('input[aria-label="Every, in minutes"]'), '30');
    await submit('Set a schedule');
    const set = mounted.posted.find((post) => post.path === '/schedules')?.body as { at: number; interval: number; source: { text: string }; recipients: unknown[] } | undefined;
    expect(set?.source.text).toBe('Read the room.');
    expect(set?.interval).toBe(1800);
    expect(set?.recipients).toEqual([holder]);
    expect(set?.at).toBeGreaterThan(Date.now() / 1000);
    expect(text()).toContain('Schedule set.');
    unmountAll();
  });
});
