/** Widgets on the canvas: what Lys holds about agents, placed by the person, fed by the lines drawn to them. */
import { act } from 'react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { $, $$, click, mount, press, settle, text, unmountAll } from './harness';
import { COURIER, SCRIBE, SERVICE, dashboard, ok, refused } from './fixtures';
import type { Route } from './fixtures';
import { mockTerminal } from './terminal-double';
import { accountsOf, combined, scopeOf, summed } from '../src/features/runtime/canvas-widgets';
import { stood } from '../src/features/runtime/canvas-marks';
import type { DashboardAgent } from '../src/features/dashboard/contract';
import { windowWords } from '../src/features/runtime/CanvasWidgets';
import { clockMs } from '../src/features/file/time';
vi.mock('@gespenst/core', () => ({ createTerminal: mockTerminal }));

const session = 'op-' + '7'.repeat(32);
const node = 'session:' + session;
const running = { session, agent: SCRIBE, machine: 'machine-one', machine_name: 'Test runner', runtime: 'sh', shown: 'running', last_reported: 'running', first_report_at: 1790000000, last_report_at: 1790000001, what: 'process 4242', stopped: null, reported_by: 'person-1', stop_asked_at: null };
const routes: Record<string, Route> = { ...SERVICE, '/runtime/live': ok({ sessions: [running], unanswered: [] }) };
const pointer = (target: Element | null, kind: string, x: number, y: number) => act(async () => { target?.dispatchEvent(new MouseEvent(kind, { bubbles: true, button: 0, clientX: x, clientY: y })); });
const kept = () => JSON.parse(localStorage.getItem('lys.canvas') ?? 'null') as { widgets: { id: string; kind: string; shows?: string; x: number; y: number }[]; links: { from: string; to: string }[] };
const widget = (kind: string) => $('[data-widget][data-kind="' + kind + '"]');
const start = (more: Record<string, unknown> = {}) => localStorage.setItem('lys.canvas', JSON.stringify({ boxes: { [node]: { x: 200, y: 200, w: 440, h: 34 } }, open: [], view: { x: 24, y: 24 }, ...more }));

/** What the pointer is over when it is let go: the test's page has no layout to ask. */
const under = (element: Element | null) => { document.elementFromPoint = () => element; };

/** Takes a kind in hand from the panel and presses the surface at (x, y). */
async function place(kind: string, x: number, y: number) {
  await click($('[data-act="draw-widget"]'));
  await click($('[data-act="place-widget"][data-kind="' + kind + '"]'));
  const surface = $('.session-canvas-scroll');
  await pointer(surface, 'pointerdown', x, y);
  await pointer(surface, 'pointerup', x, y);
}

const used = (unit: string, period: string | null, figure: number | null, unavailable: string | null = null) => ({ unit, period, since_ms: null, figure, unavailable });
const usage = (agent: string, figures: unknown[], accounts: unknown[] = []) => ({ agent, used: [], receipts: [], last_reported_ms: 1790000000000, figures, accounts });

beforeEach(() => {
  vi.stubGlobal('ResizeObserver', class { observe() {} disconnect() {} });
  localStorage.clear();
});

describe('Widgets on the canvas', () => {
  it('offers every kind by its symbol and name in a panel the address names, and places the one in hand where the canvas is pressed', async () => {
    start();
    await mount('#/canvas', routes);
    await click($('[data-act="draw-widget"]'));
    expect(location.hash).toContain('panel=widgets');
    expect($$('[data-act="place-widget"]').map((each) => each.textContent)).toEqual(['Usage', 'Budget', 'Goals', 'Requests', 'Drafts', 'Formula']);
    expect($$('[data-act="place-widget"] svg')).toHaveLength(6);
    await click($('[data-act="place-widget"][data-kind="goals"]'));
    expect($('[data-act="draw-widget"]')?.getAttribute('aria-pressed')).toBe('true');
    expect(location.hash).not.toContain('panel=');
    const surface = $('.session-canvas-scroll');
    await pointer(surface, 'pointerdown', 1024, 824);
    await pointer(surface, 'pointerup', 1024, 824);
    const goals = widget('goals') as HTMLElement;
    // No line reaches it and it sits in no box: it counts every agent.
    expect(goals.getAttribute('aria-label')).toBe('Goals of Every agent');
    expect([goals.style.left, goals.style.top]).toEqual(['1000px', '800px']);
    expect($('[data-act="draw-widget"]')?.getAttribute('aria-pressed')).toBe('false');
    expect(kept().widgets).toEqual([expect.objectContaining({ kind: 'goals', x: 1000, y: 800 })]);
    // It is a pill until its own button opens it out; pressing it only chooses it, and Delete takes the chosen thing away.
    expect([goals.dataset.view, goals.style.width, goals.style.height]).toEqual(['pill', '248px', '34px']);
    expect(goals.querySelector('.canvas-widget-figure')?.textContent).toBe('No goal');
    await pointer(goals.querySelector('.canvas-widget-bar'), 'pointerdown', 1030, 830);
    await pointer(surface, 'pointerup', 1030, 830);
    expect(goals.dataset.view).toBe('pill');
    expect($('.canvas-chosen')).not.toBeNull();
    await press('Delete');
    expect($('[data-widget]')).toBeNull();
    expect(kept().widgets).toEqual([]);
  });

  it('holds out each kind beside the agent window in front, and adds one fed by a line from that agent, each under the last', async () => {
    start();
    await mount('#/canvas', routes);
    expect($('[data-add-widget]')).toBeNull();
    await pointer($('.session-canvas-node.sessions h3'), 'pointerdown', 300, 240);
    expect($$('[data-add-widget]').map((each) => each.getAttribute('aria-label'))).toEqual(['Add Usage for Scribe', 'Add Budget for Scribe', 'Add Goals for Scribe', 'Add Requests for Scribe', 'Add Drafts for Scribe']);
    await click($('[data-add-widget="usage"]'));
    await click($('[data-add-widget="goals"]'));
    const [first, second] = [widget('usage') as HTMLElement, widget('goals') as HTMLElement];
    expect(first.getAttribute('aria-label')).toBe('Usage of Scribe');
    expect(second.getAttribute('aria-label')).toBe('Goals of Scribe');
    expect([first.style.left, first.style.top]).toEqual(['688px', '200px']);
    expect([second.style.left, second.style.top]).toEqual(['688px', '244px']);
    // The line is what feeds it, and it is kept under the agent's name so it holds when the agent starts again.
    expect(kept().links.map((link) => link.from)).toEqual(['agent:' + SCRIBE, 'agent:' + SCRIBE]);
    expect($$('path.person-line')).toHaveLength(2);
    // Pressing the canvas lets go of the window, and what it held out goes.
    await pointer($('.session-canvas-scroll'), 'pointerdown', 1200, 900);
    await pointer($('.session-canvas-scroll'), 'pointerup', 1200, 900);
    expect($('[data-add-widget]')).toBeNull();
    // With its line taken away the widget is no longer that agent's alone.
    await click($('.canvas-link-remove'));
    expect(first.getAttribute('aria-label')).toBe('Usage of Every agent');
  });

  it('counts the agents in the box it sits in until a line feeds it, and then what the lines join it to', async () => {
    start({ groups: [{ id: 'group:a', label: 'Iridium', x: 100, y: 100, w: 700, h: 500 }] });
    await mount('#/canvas', routes);
    await place('budget', 224, 324);
    expect(widget('budget')?.getAttribute('aria-label')).toBe('Budget of Iridium: 1 agent');
    // A line dragged from the window's dot and let go over the widget feeds it from that agent.
    const dot = $('.session-canvas-node.sessions [data-anchor="bottom"]');
    under(widget('budget'));
    await pointer(dot, 'pointerdown', 444, 258);
    await pointer($('.session-canvas-scroll'), 'pointermove', 300, 400);
    await pointer($('.session-canvas-scroll'), 'pointerup', 300, 400);
    expect(widget('budget')?.getAttribute('aria-label')).toBe('Budget of Scribe');
    expect(kept().links).toEqual([expect.objectContaining({ from: 'agent:' + SCRIBE, to: kept().widgets[0].id })]);
  });

  it('is dragged from the panel onto the canvas, and onto an agent window to be fed by it', async () => {
    start();
    await mount('#/canvas?panel=widgets', routes);
    const surface = $('.session-canvas-scroll') as HTMLElement;
    const drag = async (kind: string, over: Element | null, x: number, y: number) => {
      const from = $('[data-act="place-widget"][data-kind="' + kind + '"]');
      under(over);
      await pointer(from, 'pointerdown', 1100, 780);
      await pointer(from, 'pointerup', x, y);
      await click(from);
    };
    await drag('requests', surface, 524, 424);
    expect([widget('requests')?.style.left, widget('requests')?.style.top]).toEqual(['500px', '400px']);
    await drag('drafts', $('.session-canvas-node.sessions h3'), 300, 240);
    expect(widget('drafts')?.getAttribute('aria-label')).toBe('Drafts of Scribe');
    // Let go off the canvas, nothing is placed; and a drag never leaves the tool in hand.
    await drag('goals', document.body, 20, 20);
    expect(widget('goals')).toBeNull();
    expect($('[data-act="draw-widget"]')?.getAttribute('aria-pressed')).toBe('false');
  });

  it('opens out to what was reported, each figure once, and its settings choose the figure, how a level is drawn, and its colour', async () => {
    const figures = [used('context_percent', null, 43), used('tokens', 'day', 120000), used('dollars', 'day', null, 'dollars have not been reported')];
    const accounts = [{ account: 'org-main', at_ms: 1790000000000, windows: [{ duration_minutes: 300, used_percent: 28, resets_at_ms: 1790003600000 }, { duration_minutes: 10080, used_percent: 61, resets_at_ms: 1790400000000 }] }];
    start({ widgets: [{ id: 'widget:u', kind: 'usage', x: 700, y: 300, w: 340, h: 250 }], links: [{ id: 'link:u', from: 'agent:' + SCRIBE, to: 'widget:u' }] });
    await mount('#/canvas', { ...routes, '/dashboard': ok(dashboard({ [SCRIBE]: { usage: usage(SCRIBE, figures, accounts) } })) });
    const card = widget('usage') as HTMLElement;
    const turn = () => click(card.querySelector('[data-act="widget-view"]'));
    const figure = () => card.querySelector('.canvas-widget-figure')?.textContent;
    expect([card.getAttribute('aria-label'), card.dataset.view, figure()]).toEqual(['Usage of Scribe', 'pill', '43% context']);
    expect(card.querySelector('.canvas-widget-body')).toBeNull();
    // Its medium face: a few figures side by side, each its number over what it is, until the person chooses their own.
    await turn();
    expect([card.dataset.view, card.style.width, card.style.height]).toEqual(['faces', '340px', '']);
    expect($$('[data-widget] [data-stat]').map((each) => [each.dataset.stat, each.querySelector('b')?.textContent, each.querySelector('.sec')?.textContent])).toEqual([
      ['context_percent/', '43%', 'Context now'], ['tokens/day', '120,000 tokens', 'Tokens today'], ['window/300/org-main', '28%', '5-hour window of org-main, resets ' + clockMs(1790003600000)]]);
    expect(card.querySelector('.session-canvas-grip')).toBeNull();
    // Its large face: everything it holds.
    await turn();
    expect([card.dataset.view, card.style.width, card.style.height]).toEqual(['detail', '340px', '']);
    expect($$('[data-widget] tr[data-figure]').map((row) => row.textContent)).toEqual(['Context now43%', 'Tokens today120,000 tokens', 'Dollars todayNot reported']);
    expect($$('[data-widget] tr[data-account]').map((row) => row.querySelector('td')?.textContent)).toEqual(['org-main, 5-hour', 'org-main, 7-day']);
    await turn();
    expect(card.dataset.view).toBe('settings');
    expect($$('[data-variant]').map((each) => each.textContent)).toEqual(['First reported', 'Context', 'Tokens today', 'Tokens this week', 'Dollars today', 'Dollars this week', 'Running today', 'Running this week', '5-hour window', '7-day window']);
    await click($('[data-variant="window/10080"]'));
    expect([figure(), kept().widgets[0].shows]).toEqual(['61% of 7-day', 'window/10080']);
    // A level is drawn as its number, with a bar, or as a dial, on the pill and opened out.
    await click($('[data-look="bar"]'));
    expect(card.querySelector('.canvas-widget-level > span')?.getAttribute('style')).toContain('61%');
    await click($('[data-look="dial"]'));
    expect(card.querySelector('.canvas-widget-bar .canvas-dial')?.getAttribute('aria-label')).toBe('61%');
    expect((kept().widgets[0] as { look?: string }).look).toBe('dial');
    await click($('[data-colour="blue"]'));
    expect([(kept().widgets[0] as { colour?: string }).colour, card.style.getPropertyValue('--tint')]).toEqual(['blue', '#7aa7d9']);
    await turn();
    await turn();
    expect([card.dataset.view, $$('[data-widget] .canvas-stat.dial b').map((each) => each.textContent)]).toEqual(['faces', ['43%', '28%']]);
    // The figures of the medium face are chosen in its settings, each on or off, and stay in the order they are offered.
    await turn();
    await turn();
    expect($$('[data-face][aria-pressed="true"]').map((each) => each.dataset.face)).toEqual(['context_percent/', 'tokens/day', 'window/300']);
    await click($('[data-face="window/10080"]'));
    await click($('[data-face="tokens/day"]'));
    await click($('[data-face="dollars/day"]'));
    expect((kept().widgets[0] as { faces?: string[] }).faces).toEqual(['context_percent/', 'dollars/day', 'window/300', 'window/10080']);
    await turn();
    await turn();
    // A figure nobody reported is said so on the face, never shown as zero.
    expect($$('[data-widget] [data-stat]').map((each) => [each.dataset.stat, each.querySelector('b')?.textContent])).toEqual([
      ['context_percent/', '43%'], ['dollars/day', 'Not reported'], ['window/300/org-main', '28%'], ['window/10080/org-main', '61%']]);
    expect($('[data-stat="dollars/day"]')?.getAttribute('title')).toBe('Scribe: dollars have not been reported');
    // With nothing but an unreported figure shown there is no level to draw; with no figure chosen the face says so.
    await turn();
    await turn();
    await click($('[data-variant="dollars/day"]'));
    for (const key of ['context_percent/', 'window/300', 'window/10080']) await click($('[data-face="' + key + '"]'));
    expect([figure(), $('[data-look]')]).toEqual(['Not reported', null]);
    await click($('[data-face="dollars/day"]'));
    expect((kept().widgets[0] as { faces?: string[] }).faces).toEqual([]);
    await turn();
    await turn();
    expect([card.dataset.view, card.querySelector('.canvas-widget-body')?.textContent]).toEqual(['faces', 'No figure is chosen for this face.']);
    await turn();
    await turn();
    await click($('[data-variant=""]'));
    expect(kept().widgets[0].shows).toBeUndefined();
    // Locked, it is not opened, moved or taken away; unlocked, it is again.
    await click(card.querySelector('[data-act="widget-lock"]'));
    expect([(kept().widgets[0] as { locked?: boolean }).locked, (card.querySelector('[data-act="widget-view"]') as HTMLButtonElement).disabled, $('[data-act="remove-widget"]')]).toEqual([true, true, null]);
    await pointer(card.querySelector('.canvas-widget-bar'), 'pointerdown', 710, 310);
    await pointer($('.session-canvas-scroll'), 'pointermove', 900, 500);
    await pointer($('.session-canvas-scroll'), 'pointerup', 900, 500);
    expect([card.style.left, card.style.top]).toEqual(['700px', '300px']);
    await click(card.querySelector('[data-act="widget-lock"]'));
    expect((kept().widgets[0] as { locked?: boolean }).locked).toBeUndefined();
    // A press with the other button opens the settings from any view, and the small cross there takes the widget away.
    await turn();
    expect(card.dataset.view).toBe('pill');
    await act(async () => { card.dispatchEvent(new MouseEvent('contextmenu', { bubbles: true, cancelable: true })); });
    expect(card.dataset.view).toBe('settings');
    await click($('[data-act="remove-widget"]'));
    expect($('[data-widget]')).toBeNull();
  });

  it('combines the figures of the widgets a line joins a formula to, the way the person chose, and says when they cannot be combined', async () => {
    const tokens = (id: string, y: number) => ({ id, kind: 'usage', shows: 'tokens/day', x: 700, y, w: 340, h: 250 });
    start({
      widgets: [tokens('widget:s', 100), tokens('widget:c', 150), { id: 'widget:g', kind: 'goals', x: 700, y: 200, w: 340, h: 190 }, { id: 'widget:f', kind: 'formula', x: 1100, y: 100, w: 320, h: 150 }, { id: 'widget:t', kind: 'formula', shows: 'highest', x: 1100, y: 300, w: 320, h: 150 },
        { id: 'widget:m', kind: 'formula', x: 1100, y: 500, w: 320, h: 150 }, { id: 'widget:n', kind: 'formula', x: 1100, y: 600, w: 320, h: 150 }],
      // One usage widget is fed by Scribe; the other, fed by no line and in no box, counts every agent.
      links: [{ id: 'l1', from: 'agent:' + SCRIBE, to: 'widget:s' }, { id: 'l3', from: 'widget:s', to: 'widget:f' }, { id: 'l4', from: 'widget:f', to: 'widget:c' },
        { id: 'l6', from: 'widget:s', to: 'widget:m' }, { id: 'l7', from: 'widget:g', to: 'widget:m' },
        // A formula feeds another only by a line drawn from it: the second is fed by the first, and never the first by the second.
        { id: 'l5', from: 'widget:f', to: 'widget:t' }],
    });
    await mount('#/canvas', { ...routes, '/dashboard': ok(dashboard({ [SCRIBE]: { usage: usage(SCRIBE, [used('tokens', 'day', 120000)]) }, [COURIER]: { usage: usage(COURIER, [used('tokens', 'day', 30000)]) } })) });
    const card = (id: string) => $('[data-widget="' + id + '"]') as HTMLElement;
    const figure = (id: string) => card(id).querySelector('.canvas-widget-figure')?.textContent;
    // Fed whichever way the line was drawn; it adds until the person chooses another way.
    expect([card('widget:f').getAttribute('aria-label'), figure('widget:f'), figure('widget:t')]).toEqual(['Formula of 2 inputs', '270,000 tokens', '270,000 tokens']);
    // Tokens and a count of goals are not one number; a formula nothing feeds says so.
    expect([figure('widget:m'), figure('widget:n'), card('widget:n').getAttribute('aria-label')]).toEqual(['Mixed units', 'No input', 'Formula of 0 inputs']);
    // It has no medium face: its button opens it out to what feeds it, then its settings.
    await click(card('widget:f').querySelector('[data-act="widget-view"]'));
    expect(card('widget:f').dataset.view).toBe('detail');
    expect($$('[data-widget="widget:f"] tr[data-input]').map((row) => row.textContent)).toEqual(['Usage of Scribe120,000 tokens today', 'Usage of Every agent150,000 tokens today']);
    expect($('[data-widget="widget:f"] tr[data-result]')?.textContent).toBe('Sum270,000 tokens');
    await click(card('widget:f').querySelector('[data-act="widget-view"]'));
    expect($$('[data-widget="widget:f"] [data-variant]').map((each) => [each.textContent, each.getAttribute('aria-pressed')])).toEqual([['Sum', 'true'], ['Average', 'false'], ['Highest', 'false'], ['Lowest', 'false'], ['How many', 'false']]);
    expect([$('[data-widget="widget:f"] [data-face]'), $('[data-widget="widget:f"] a[href]')]).toEqual([null, null]);
    await click($('[data-widget="widget:f"] [data-variant="average"]'));
    expect([figure('widget:f'), figure('widget:t'), kept().widgets.find((each) => each.id === 'widget:f')?.shows]).toEqual(['135,000 tokens', '135,000 tokens', 'average']);
    await click($('[data-widget="widget:f"] [data-variant="lowest"]'));
    expect(figure('widget:f')).toBe('120,000 tokens');
    await click($('[data-widget="widget:f"] [data-variant="count"]'));
    expect(figure('widget:f')).toBe('2');
    // No widget beside an agent's window offers a formula: it is fed by widgets, not by an agent.
    await click($('.session-canvas-node.sessions h3'));
    expect($('[data-add-widget="formula"]')).toBeNull();
  });

  it('says by name what could not be read, and that a kind is not known', async () => {
    start({ widgets: [{ id: 'widget:b', kind: 'weather', x: 700, y: 520, w: 340, h: 180, view: 'detail' }] });
    await mount('#/canvas', routes);
    expect(widget('weather')?.textContent).toContain('This page does not know a widget of the kind “weather”.');
    document.body.innerHTML = '';
    start({ widgets: [{ id: 'widget:a', kind: 'budget', x: 700, y: 300, w: 340, h: 170 }] });
    await mount('#/canvas', { ...routes, '/dashboard': refused(503, 'DashboardUnavailable', 'the dashboard is closed') });
    expect(widget('budget')?.querySelector('.canvas-widget-figure')?.textContent).toBe('DashboardUnavailable');
    expect(text()).not.toContain('No limit');
  });

  it('gives a box and a note the next colour each time their dot is pressed, and keeps it', async () => {
    start({ groups: [{ id: 'group:a', label: 'Iridium', x: 100, y: 100, w: 700, h: 500 }], notes: [{ id: 'note:a', text: 'Ask', x: 900, y: 100, w: 260, h: 180, colour: 'grey' }] });
    await mount('#/canvas', routes);
    const keptMarks = () => JSON.parse(localStorage.getItem('lys.canvas') ?? '{}') as { groups: { colour?: string }[]; notes: { colour?: string }[] };
    await click($('.canvas-group [data-act="colour"]'));
    expect([keptMarks().groups[0].colour, ($('.canvas-group') as HTMLElement).style.getPropertyValue('--tint')]).toEqual(['green', '#74b584']);
    // After the last colour comes Lys's own again: no colour is kept.
    await click($('.canvas-note [data-act="colour"]'));
    expect(keptMarks().notes[0].colour).toBeUndefined();
  });
});

describe('Typing to several agents at once', () => {
  /** Writes into the form's one field, the way typing does. */
  const write = (value: string) => act(async () => {
    const field = $('.canvas-together textarea') as HTMLTextAreaElement;
    Object.getOwnPropertyDescriptor(HTMLTextAreaElement.prototype, 'value')?.set?.call(field, value);
    field.dispatchEvent(new Event('input', { bubbles: true }));
  });
  it('types what was written once into each chosen agent\'s terminal, then Enter, and says by name who it did not reach', async () => {
    const other = { ...running, session: 'op-' + '8'.repeat(32), agent: COURIER };
    const sent: Record<string, number[][]> = {};
    const input = (session: string, answer?: Route): Record<string, Route> => ({ ['POST /runtime/sessions/' + session + '/input-bytes']: answer ?? ((body) => {
      (sent[session] ??= []).push((body as { data: number[] }).data);
      return ok({ session, answer: { kind: 'delivered', session }, receipt: { index: 7 } });
    }) });
    start();
    await mount('#/canvas', { ...routes, '/runtime/live': ok({ sessions: [running, other], unanswered: [] }), ...input(session), ...input(other.session) });
    const bar = (id: string) => $('[data-node="session:' + id + '"] .session-canvas-bar');
    const hold = (target: Element | null) => act(async () => { target?.dispatchEvent(new MouseEvent('pointerdown', { bubbles: true, button: 0, metaKey: true, clientX: 300, clientY: 210 })); });
    expect($('.canvas-together')).toBeNull();
    // A press on a window's bar with Command held chooses it; the form says who it types to, and the window shows it is chosen.
    await hold(bar(session));
    expect([$('.canvas-together')?.getAttribute('aria-label'), $('[data-node="' + node + '"]')?.className.includes('together')]).toEqual(['Type to 1 agent', true]);
    await click($('[data-act="together-all"]'));
    expect([$$('[data-together]').map((each) => each.textContent), $('[data-act="together-all"]')]).toEqual([['Scribe×', 'Courier×'], null]);
    await write('Run the tests.');
    await act(async () => { $('.canvas-together textarea')?.dispatchEvent(new KeyboardEvent('keydown', { key: 'Enter', bubbles: true })); });
    await settle();
    const words = Array.from(new TextEncoder().encode('Run the tests.'));
    expect(sent).toEqual({ [session]: [words, [13]], [other.session]: [words, [13]] });
    expect([$('.canvas-together [role="status"]')?.textContent, ($('.canvas-together textarea') as HTMLTextAreaElement).value]).toEqual(['Typed to 2 agents.', '']);
    // One left out is not typed to; pressed again with Command held, a window leaves; Done puts the form away.
    await click($('[data-together="session:' + other.session + '"] button'));
    expect($$('[data-together]').map((each) => each.textContent)).toEqual(['Scribe×']);
    await hold(bar(session));
    expect($('.canvas-together')).toBeNull();
    // An agent it did not reach is said by name with why, and the words stay to be sent again by the person, never by the page.
    unmountAll();
    document.body.innerHTML = '';
    await mount('#/canvas', { ...routes, '/runtime/live': ok({ sessions: [running, other], unanswered: [] }), ...input(session), ...input(other.session, refused(409, 'RuntimeSessionEnded', 'the session has ended')) });
    await hold(bar(session));
    await hold(bar(other.session));
    await write('Stop.');
    await click($('.canvas-together button[type="submit"]'));
    await settle();
    expect($('.canvas-together [role="alert"]')?.textContent).toContain('Not typed to Courier: ');
    expect($('.canvas-together [role="alert"]')?.textContent).toContain('Typed to Scribe.');
    expect(($('.canvas-together textarea') as HTMLTextAreaElement).value).toBe('Stop.');
    await click($('[data-act="together-clear"]'));
    expect($('.canvas-together')).toBeNull();
  });
});

describe('Looking closer', () => {
  it('brings the view in to a window when its bar is double-pressed, and goes back to the view before when it is double-pressed again', async () => {
    start();
    await mount('#/canvas', routes);
    const surface = $('.session-canvas-scroll') as HTMLElement;
    Object.defineProperty(surface, 'clientWidth', { configurable: true, value: 1008 });
    Object.defineProperty(surface, 'clientHeight', { configurable: true, value: 800 });
    const drawing = $('.session-canvas') as HTMLElement;
    const before = drawing.style.transform;
    const twice = () => act(async () => { $('.session-canvas-node.sessions .session-canvas-bar')?.dispatchEvent(new MouseEvent('dblclick', { bubbles: true })); });
    await twice();
    // The window is 440 wide: with its margin it fills the 1,008 of the view at twice its size, in the middle.
    expect(drawing.style.transform).toBe('translate(-336px, -34px) scale(2)');
    await twice();
    expect(drawing.style.transform).toBe(before);
  });
});

describe('What a widget counts', () => {
  const row = (id: string, name: string, more: Record<string, unknown> = {}) => ({ agent: { id, display_name: name, state: 'active' }, teams: [], sessions: [], usage: usage(id, []), budget: {}, goals: { goals: [] }, ...more }) as unknown as DashboardAgent;
  const rows = [row(SCRIBE, 'Scribe'), row(COURIER, 'Courier'), row('agent-x', 'Xavier')];
  const windows = [{ id: 'session:1', agent: SCRIBE, box: { x: 100, y: 100, w: 100, h: 40 } }, { id: 'session:2', agent: COURIER, box: { x: 900, y: 100, w: 100, h: 40 } }];
  const box = { id: 'group:a', label: 'Iridium', x: 0, y: 0, w: 500, h: 500 };
  const there = { id: 'widget:w', kind: 'usage', x: 2000, y: 2000, w: 100, h: 100 };

  it('is every agent with no line and no box, the box it sits in, or exactly what its lines join it to', () => {
    expect(scopeOf(there, [], [box], windows, rows)).toEqual({ rows, whose: 'Every agent', everyone: true });
    expect(scopeOf({ ...there, x: 200, y: 200 }, [], [box], windows, rows)).toMatchObject({ rows: [rows[0]], whose: 'Iridium: 1 agent', everyone: false });
    // A line from a window, by the name it has now or the agent's own; a line to a box counts the agents in the box.
    expect(scopeOf(there, [{ id: 'l', from: 'session:2', to: 'widget:w' }], [box], windows, rows)).toMatchObject({ rows: [rows[1]], whose: 'Courier' });
    expect(scopeOf(there, [{ id: 'l', from: 'widget:w', to: 'agent:' + SCRIBE }, { id: 'm', from: 'session:2', to: 'widget:w' }], [box], windows, rows)).toMatchObject({ rows: [rows[0], rows[1]], whose: 'Scribe, Courier' });
    expect(scopeOf(there, [{ id: 'l', from: 'group:a', to: 'widget:w' }], [box], windows, rows)).toMatchObject({ rows: [rows[0]], whose: 'Iridium: 1 agent' });
    // A line to a note, or to a window that is gone, feeds nothing: the widget falls back to where it sits.
    expect(scopeOf(there, [{ id: 'l', from: 'note:n', to: 'widget:w' }], [box], windows, rows).everyone).toBe(true);
  });

  it('moves the widgets an opened one would cover down, clear of it, and leaves the rest where they were put', () => {
    const pill = (id: string, x: number, y: number) => ({ id, kind: 'goals', x, y, w: 340, h: 190 });
    const open = { id: 'open', kind: 'usage', x: 100, y: 100, w: 340, h: 200, view: 'detail' as const };
    const places = (widgets: Parameters<typeof stood>[0]) => stood(widgets).map((each) => [each.id, each.y, each.h]);
    // Two pills stacked under an opened widget go below it, the second below the first; one beside it stays.
    expect(places([open, pill('a', 100, 144), pill('b', 100, 188), pill('beside', 500, 144), pill('above', 100, 40)])).toEqual([
      ['open', 100, 200], ['a', 310, 34], ['b', 354, 34], ['beside', 144, 34], ['above', 40, 34]]);
    // As pills, each stands where the person put it, even one laid over another.
    expect(places([{ ...open, view: undefined }, pill('a', 100, 110)])).toEqual([['open', 100, 34], ['a', 110, 34]]);
  });

  it('combines figures of one kind and refuses to make one number of different kinds', () => {
    const tokens = (amount: number) => ({ amount, unit: 'tokens' });
    expect(['sum', 'average', 'highest', 'lowest', 'count'].map((by) => combined(by, [tokens(10), tokens(40), tokens(100)]))).toEqual(
      [tokens(150), tokens(50), tokens(100), tokens(10), { amount: 3, unit: 'count' }]);
    expect(combined('sum', [tokens(10), { amount: 2, unit: 'dollars' }])).toBe('mixed');
    expect([combined('sum', []), combined('count', [])]).toEqual(['none', { amount: 0, unit: 'count' }]);
  });

  it('adds amounts over the agents, shows levels at their highest, and counts who did not report with the reason', () => {
    const both = [
      row(SCRIBE, 'Scribe', { usage: usage(SCRIBE, [used('tokens', 'day', 100), used('context_percent', null, 40), used('dollars', 'day', null, 'no dollars')]) }),
      row(COURIER, 'Courier', { usage: usage(COURIER, [used('tokens', 'day', 50), used('context_percent', null, 70), used('dollars', 'day', 2.5)]) }),
      row('agent-x', 'Xavier', { usage: { refusal: 'UsageUnavailable', reason: 'closed' } }),
    ];
    expect(summed(both)).toEqual([
      { unit: 'tokens', period: 'day', figure: 150, reported: 2, missing: [] },
      { unit: 'context_percent', period: null, figure: 70, reported: 2, missing: [] },
      { unit: 'dollars', period: 'day', figure: 2.5, reported: 1, missing: ['Scribe: no dollars'] },
    ]);
  });

  it('keeps each account once, at its latest report, and names a window by its length', () => {
    const window = (percent: number) => [{ duration_minutes: 300, used_percent: percent, resets_at_ms: 5 }];
    const both = [
      row(SCRIBE, 'Scribe', { usage: usage(SCRIBE, [], [{ account: 'org-b', at_ms: 10, windows: window(20) }, { account: 'org-a', at_ms: 10, windows: window(5) }]) }),
      row(COURIER, 'Courier', { usage: usage(COURIER, [], [{ account: 'org-b', at_ms: 30, windows: window(35) }]) }),
    ];
    expect(accountsOf(both).map((each) => [each.account, each.windows[0].used_percent])).toEqual([['org-a', 5], ['org-b', 35]]);
    expect([300, 10080, 90, 45].map(windowWords)).toEqual(['5-hour', '7-day', '90-minute', '45-minute']);
  });
});
