/**
 * The 15 October review of the Dashboard, an agent's Limits and goals, its file and the shell: what a person saw that
 * they should not have, each proved gone. Layout jsdom cannot measure is held by the rules that cause it.
 */
import { readFileSync } from 'node:fs';
import { describe, expect, it } from 'vitest';
import { $, $$, click, mount, text } from './harness';
import { ADA, SCRIBE, SERVICE, ok } from './fixtures';
import type { Route } from './fixtures';
import { budgetsView } from './budget-fixtures';

const css = (file: string) => readFileSync(new URL('../src/' + file, import.meta.url), 'utf8');
const rule = (sheet: string, selector: string) => new RegExp('(^|\\n)' + selector.replace(/[.*+?^${}()|[\]\\]/g, '\\$&') + '\\s*\\{([^}]*)\\}').exec(sheet)?.[2] ?? '';
const holder = { kind: 'agent' as const, id: SCRIBE };
const now = Math.floor(Date.now() / 1000);

describe('Dashboard layout', () => {
  it('lets the widgets row take its whole height, so a second row of cards never runs over the first', () => {
    const widgets = rule(css('features/dashboard/dashboard.css'), '.dash-widgets');
    expect(widgets).toContain('grid-column: 1 / -1');
    // A grid item with min-height 0 lets its auto row shrink to the space left, and the cards spill over each other.
    expect(widgets).not.toMatch(/min-height:\s*0/);
  });

  it('wraps an agent’s limits in their column instead of pushing Goals out of the panel', () => {
    const sheet = css('features/dashboard/dashboard.css');
    expect(rule(sheet, '.you-tree')).not.toContain('max-content');
    expect(rule(sheet, '.you-tree td.you-budget')).toContain('max-width');
  });
});

describe('An agent’s file', () => {
  it('keeps its state and menu at the right of its head, not in the middle of the page', async () => {
    await mount('#/file/' + SCRIBE, SERVICE);
    const actions = $('.file .head > .head-actions');
    expect(actions?.querySelector('#state')).not.toBeNull();
    expect(actions?.getAttribute('style')).toBeNull();
  });

  it('says a person’s access in words, without the inner state names or a note only an engineer reads', async () => {
    await mount('#/file/' + ADA, SERVICE);
    const facts = $('.grid2 dl.facts')?.textContent ?? '';
    expect(facts).toContain('AccessActive');
    expect(facts).not.toContain('authority only');
    expect(facts).not.toContain('A person answers for themselves');
    expect($$('.grid2 .row .dim').map((each) => each.textContent)).toEqual(['Active', 'Not turned on yet', 'Paused']);
  });

  it('offers a proof of each change by name rather than a log entry number, and no lifecycle diagram', async () => {
    await mount('#/file/' + SCRIBE + '/record', { ...SERVICE, ['/agents/' + SCRIBE + '/stops']: ok({ stops: [] }) });
    expect(text()).not.toContain('registered → active');
    expect(text()).not.toContain('entry #');
    expect($$('a[download^="lys-change-"]').map((a) => a.textContent)).toEqual(['Download proof of change 4', 'Download proof of change 5']);
  });
});

/** The Limits and goals tab of Scribe, with one long goal, a variable set by Ada, and a schedule. */
function limits(extra: Record<string, Route> = {}): Record<string, Route> {
  return {
    ...SERVICE,
    ['/budgets/agent/' + SCRIBE]: ok(budgetsView(holder)),
    ['/agents/' + SCRIBE + '/usage']: ok({ agent: SCRIBE, used: [], receipts: [], last_reported_ms: 1790000000000 }),
    ['/agents/' + SCRIBE + '/goals']: ok({ goals: [{ goal: { id: 'op-' + 'a'.repeat(32), kind: 'deliverable', words: 'Keep the directory available to every ward during the night shift, including the handover at seven', deadline: null, active: true, evidence: null }, standing: 'open' }] }),
    ['/agents/' + SCRIBE + '/words']: ok({ agent: SCRIBE, resolved: [{ slot: 'compaction', text: '/compact', source: 'agent', contributed: [] }], held: { compaction: { setting: { kind: 'text', text: '/compact' }, revision: 3, by: ADA, at: now } } }),
    ['/agents/' + SCRIBE + '/variables']: ok({ scope: holder, revision: 4, values: { ward: { value: 'ward-7', revision: 2, author: ADA, at: now } }, expired: [] }),
    '/schedules': ok({ schedules: [{ schedule: { id: 'op-' + 'e'.repeat(32), at: now - 60, interval: 1800, recipients: [holder], source: { kind: 'text', text: 'Check the handover notes' }, author: ADA, set_at: now - 120 }, fired: [], next_due: now + 1800 }] }),
    ...extra,
  };
}

describe('An agent’s Limits and goals', () => {
  it('shows a whole goal in a box that grows with it, and its kind in words', async () => {
    await mount('#/file/' + SCRIBE + '/budgets', limits());
    const box = $('section.usage-goals tbody textarea[name="words"]');
    expect(box?.className).toContain('usage-grow');
    expect((box as HTMLTextAreaElement | null)?.value).toContain('handover at seven');
    expect($('section.usage-goals tbody td')?.textContent).toBe('Deliverable');
    // What a screen reader says of the goal is its words, never its identifier.
    expect($$('section.usage-goals [aria-label]').map((each) => each.getAttribute('aria-label')).join(' ')).not.toContain('op-' + 'a'.repeat(32));
  });

  it('names who set a message and a variable by name, never by their identifier, and asks no time in seconds since the epoch', async () => {
    await mount('#/file/' + SCRIBE + '/budgets', limits());
    for (const section of ['section.usage-words', 'section.usage-variables']) {
      expect($(section)?.textContent).toContain('Ada (test person)');
      expect($(section)?.textContent).not.toContain(ADA);
    }
    expect($('section.usage-variables')?.textContent).not.toMatch(/revision/i);
    const until = $('input[name="expires_at"]') as HTMLInputElement | null;
    expect(until?.type).toBe('datetime-local');
    expect(until?.getAttribute('aria-label')).not.toContain('epoch');
    for (const name of ['name', 'value']) expect($('input[form="set-variable"][name="' + name + '"]')?.getAttribute('placeholder')).toBeTruthy();
    for (const name of ['text', 'interval', 'max_occurrences']) expect($('input[form="set-schedule"][name="' + name + '"]')?.getAttribute('placeholder')).toBeTruthy();
  });

  it('says why a variable cannot be set, rather than greying Set with no reason', async () => {
    await mount('#/file/' + SCRIBE + '/budgets', limits());
    const name = $('input[form="set-variable"][name="name"]') as HTMLInputElement;
    const { type } = await import('./harness');
    await type(name, 'Ward List');
    expect($('section.usage-variables [role="alert"]')?.textContent).toContain('lower-case');
    expect(($('button[form="set-variable"]') as HTMLButtonElement).disabled).toBe(true);
  });

  it('asks before stopping a schedule for good, and names it by what it sends, never its identifier', async () => {
    const { posted } = await mount('#/file/' + SCRIBE + '/budgets', limits());
    const stop = $('section[aria-label="Schedules"] button[aria-label="Stop the schedule “Check the handover notes”"]');
    expect(stop).not.toBeNull();
    await click(stop);
    expect(posted.filter((each) => each.path.endsWith('/stop'))).toEqual([]);
    expect($('section[aria-label="Schedules"] .usage-confirm')?.textContent).toContain('Stop it for good?');
    await click($('button[aria-label="Keep the schedule"]'));
    expect($('section[aria-label="Schedules"] .usage-confirm')).toBeNull();
    expect($('section[aria-label="Schedules"]')?.innerHTML).not.toContain('aria-label="Stop schedule op-');
  });

  it('says a panel it could not read in words, with the refusal kept under Error details', async () => {
    await mount('#/file/' + SCRIBE + '/budgets', limits({ ['/agents/' + SCRIBE + '/variables']: { status: 503, body: { refusal: 'VariablesUnavailable', reason: 'the store is closed' } } }));
    const face = [...document.querySelectorAll('.usage > .card[role="status"]')].find((card) => card.textContent?.includes('The variables could not be read.'));
    expect(face).toBeDefined();
    expect(face?.querySelector('details .refusal-name')?.textContent).toBe('VariablesUnavailable: the store is closed');
    expect(face?.querySelector('p.why-not')?.textContent).toBe('The variables could not be read.');
  });
});

describe('The shell', () => {
  it('offers the Dashboard from a page that is not found, and does not show the address as code', async () => {
    await mount('#/no-such-page', SERVICE);
    expect(text()).toContain('Not found');
    expect($('a.btn[href="#/"]')?.textContent).toBe('Go Home');
    expect($('.page p.code')).toBeNull();
  });
});
