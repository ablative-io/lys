import { act } from 'react';
import { describe, expect, it, vi } from 'vitest';
import { GO } from '../src/shell/keys';
import { CONCEPTS, TRACKS, conceptById, conceptGroups } from '../src/shell/help/concepts';
import { PAGES, RAIL_REGION, pageKeyOf } from '../src/shell/help/pages';
import { goShortcuts } from '../src/shell/help/shortcuts';
import { CLOSED_HELP, helpStateFromParams, readHelpUrl, withHelpState } from '../src/shell/help/url';
import { $, $$, click, mount, press, settle, type, unreachable } from './harness';

// jsdom lays nothing out; give every element a box on screen so the page help can outline its regions.
function layOut(left = 100) {
  vi.spyOn(Element.prototype, 'getBoundingClientRect').mockReturnValue({
    x: left, y: 100, left, top: 100, right: left + 80, bottom: 120, width: 80, height: 20, toJSON: () => ({}),
  } as DOMRect);
}

/** The notes listed in the panel, in order, by title. */
const notes = () => $$('#helpLayer [data-help-note]').map((n) => n.querySelector('.help-call-t')?.textContent?.replace(/^\d+/, ''));
const present = (key: string) => [...PAGES[key].regions, RAIL_REGION].filter((r) => $$(r.sel).some((e) => !e.closest('#helpLayer'))).map((r) => r.title);

/** One animation frame: the page help re-measures a frame after the screen's content changes. */
async function frame() {
  await act(async () => { await new Promise<void>((resolve) => requestAnimationFrame(() => resolve())); });
}

async function go(hash: string) {
  await act(async () => {
    location.hash = hash;
    window.dispatchEvent(new HashChangeEvent('hashchange'));
  });
  await settle();
}

describe('the page help', () => {
  it('opens on ?, numbers every region on screen and skips the ones that are not', async () => {
    layOut();
    await mount('#/people');
    await press('?', {}, document.body);
    expect($('#helpLayer')).not.toBeNull();
    const shown = present('people');
    expect(shown).toContain('The list');
    expect(shown).toContain('The rail');
    // the Add buttons sit in the header on this page; a region with no element is left out, never drawn empty
    const absent = PAGES.people.regions.filter((r) => !$$(r.sel).length).map((r) => r.title);
    expect(notes()).toEqual(shown);
    for (const title of absent) expect(notes()).not.toContain(title);
    expect($$('#helpLayer .help-reg .help-n').map((n) => n.textContent)).toEqual(shown.map((_, i) => String(i + 1)));
    expect($('[role="dialog"][aria-label="Help for People and agents"]')).not.toBeNull();
    expect(readHelpUrl()).toEqual({ page: true, topic: null });
  });

  it('toggles on ⌘/ and on Ctrl+/ as on ?', async () => {
    layOut();
    await mount('#/people');
    await press('/', { metaKey: true }, document.body);
    expect($('#helpLayer')).not.toBeNull();
    await press('/', { metaKey: true }, document.body);
    expect($('#helpLayer')).toBeNull();
    await press('/', { ctrlKey: true }, document.body);
    expect($('#helpLayer')).not.toBeNull();
    await press('?', {}, document.body);
    expect($('#helpLayer')).toBeNull();
    expect(location.search).toBe('');
  });

  it('never opens from a key typed into a field', async () => {
    await mount('#/people/new');
    const field = $('input[name="email"]');
    expect(field).not.toBeNull();
    await press('?', {}, field);
    expect($('#helpLayer')).toBeNull();
  });

  it('opens on ⌘/ even from inside a field, where ? is only a character', async () => {
    layOut();
    await mount('#/people/new');
    const field = $('input[name="email"]');
    await press('/', { metaKey: true }, field);
    expect($('#helpLayer')).not.toBeNull();
    await press('/', { metaKey: true }, field);
    expect($('#helpLayer')).toBeNull();
  });

  it('on a phone lists the same numbered notes, and nothing floats over the page', async () => {
    layOut();
    const width = innerWidth;
    Object.defineProperty(window, 'innerWidth', { configurable: true, value: 390 });
    try {
      await mount('#/people');
      await press('?', {}, document.body);
      expect($$('#helpLayer .help-call')).toEqual([]);
      expect(notes()).toEqual(present('people'));
      expect(notes()).toContain('The list');
    } finally {
      Object.defineProperty(window, 'innerWidth', { configurable: true, value: width });
    }
  });


  it('closes on Escape and gives focus back to where it was', async () => {
    layOut();
    await mount('#/people');
    const row = $('tr[data-pick="1"]');
    row?.focus();
    await press('?');
    expect($('#helpLayer')).not.toBeNull();
    expect(document.activeElement?.getAttribute('role')).toBe('dialog');
    await press('Escape');
    expect($('#helpLayer')).toBeNull();
    expect(document.activeElement).toBe(row);
    expect(location.hash).toBe('#/people');
    expect(readHelpUrl()).toEqual(CLOSED_HELP);
  });

  it('closes on the dim or the close button, and never presses what is underneath', async () => {
    layOut();
    await mount('#/people');
    const pressed = vi.fn();
    document.addEventListener('click', pressed);
    await press('?', {}, document.body);
    await click($('[data-help-dim]'));
    expect($('#helpLayer')).toBeNull();
    expect(pressed).not.toHaveBeenCalled();
    await press('?', {}, document.body);
    await click($('[data-help-close]'));
    expect($('#helpLayer')).toBeNull();
    document.removeEventListener('click', pressed);
  });

  it('follows the page: moving screens shows the new page’s regions and keys', async () => {
    layOut();
    await mount('#/dashboard');
    await press('?', {}, document.body);
    expect($('#helpLayer [role="dialog"]')?.getAttribute('aria-label')).toBe('Help for Home');
    await go('#/network');
    await frame();
    expect($('#helpLayer')).not.toBeNull();
    expect($('#helpLayer [role="dialog"]')?.getAttribute('aria-label')).toBe('Help for Computers');
    expect(notes()).toEqual(present('network'));
  });

  it('lists the shell’s real keys: every g letter, by its screen’s name', async () => {
    layOut();
    await mount('#/people');
    await press('?', {}, document.body);
    const keys = $$('#helpLayer .help-kt dt').map((k) => k.textContent);
    for (const letter of Object.keys(GO)) expect(keys.some((k) => k?.split(' or ').includes('g ' + letter))).toBe(true);
    expect(keys).toContain('? (outside a field) or ⌘/');
    // The page's own list keys are the shell's, said once.
    expect(keys).toContain('j / k, Enter');
    expect(keys).not.toContain('j / k');
    expect(new Set(keys).size).toBe(keys.length);
  });

  it('re-measures on a resize the observer reports, with no timer', async () => {
    let left = 100;
    vi.spyOn(Element.prototype, 'getBoundingClientRect').mockImplementation(
      () => ({ x: left, y: 100, left, top: 100, right: left + 80, bottom: 120, width: 80, height: 20, toJSON: () => ({}) }) as DOMRect,
    );
    const made: { fire: () => void; observed: Element[] }[] = [];
    vi.stubGlobal('ResizeObserver', class {
      observed: Element[] = [];
      constructor(private readonly callback: ResizeObserverCallback) { made.push(this as unknown as { fire: () => void; observed: Element[] }); }
      observe(el: Element) { this.observed.push(el); }
      unobserve(el: Element) { this.observed = this.observed.filter((e) => e !== el); }
      disconnect() { this.observed = []; }
      fire() { act(() => this.callback([], this as unknown as ResizeObserver)); }
    });
    await mount('#/people');
    await press('?', {}, document.body);
    expect(made).toHaveLength(1);
    expect(made[0].observed).toContain(document.documentElement);
    expect(made[0].observed).toContain($('#rail'));
    const rail = () => $('#helpLayer [data-help-region="The rail"]');
    expect(rail()?.style.left).toBe('100px');
    vi.useFakeTimers();
    try {
      left = 300;
      made[0].fire();
      expect(rail()?.style.left).toBe('300px');
      expect(vi.getTimerCount()).toBe(0);
    } finally {
      vi.useRealTimers();
    }
    await press('Escape');
    expect(made[0].observed).toEqual([]);
  });

  it('opens the full help on this page’s sheet', async () => {
    layOut();
    await mount('#/people');
    await press('?', {}, document.body);
    await click($('[data-help-full]'));
    expect($('#helpLayer')).toBeNull();
    expect($('#dock')?.classList.contains('open')).toBe(true);
    expect($('.dock-head b')?.textContent).toBe(conceptById(PAGES.people.topic)?.title);
    expect(readHelpUrl()).toEqual({ page: false, topic: PAGES.people.topic });
  });

  it('is not offered to someone signed out', async () => {
    await mount('#/sign-in');
    await press('?', {}, document.body);
    expect($('#helpLayer')).toBeNull();
  });
});

describe('the full help', () => {
  it('opens from the rail\'s Help, through Full help, on an index of every topic, by track, usable by keyboard', async () => {
    await mount('#/people');
    await click($('[data-dockbtn="help"]'));
    // Help is this screen explained first; every topic is the layer's Full help.
    expect($('#helpLayer')).not.toBeNull();
    expect($('#dock.open')).toBeNull();
    await click($('[data-help-full]'));
    expect($('#helpLayer')).toBeNull();
    // Full help opens on this page's own topic; Back goes to every topic.
    expect(readHelpUrl().topic).toBe(PAGES[pageKeyOf('/people')].topic);
    await click($('[data-help=""]'));
    expect($$('#helpList .concept').length).toBe(CONCEPTS.length);
    expect($$('#helpList .help-track').map((t) => t.dataset.track)).toEqual(TRACKS.map(([id]) => id));
    expect(unreachable()).toEqual([]);
    const concept = $('#helpList .concept[data-help="start.agent"]');
    concept?.focus();
    await press('Enter');
    expect($('.dock-head b')?.textContent).toBe('Add an agent');
    expect($$('.help-steps li').length).toBe(conceptById('start.agent')?.steps.length);
    expect(readHelpUrl().topic).toBe('start.agent');
    await click($('[data-help=""]'));
    expect($('#helpQ')).not.toBeNull();
    expect(readHelpUrl().topic).toBe('index');
    await click($('[data-dockbtn-close]'));
    expect($('#dock')?.classList.contains('open')).toBe(false);
    expect(readHelpUrl()).toEqual(CLOSED_HELP);
  });

  it('filters the index by every word typed, and says when nothing matches', async () => {
    await mount('#/people');
    await click($('[data-dockbtn="help"]'));
    await click($('[data-help-full]'));
    await click($('[data-help=""]'));
    await type($('#helpQ'), 'spending limit');
    const shown = $$('#helpList .concept').map((c) => c.dataset.help);
    expect(shown).toEqual(conceptGroups('spending limit').flatMap((g) => g.concepts.map((c) => c.id)));
    expect(shown).toContain('budgets.limit');
    expect(shown).not.toContain('computers.network');
    await type($('#helpQ'), 'zzqq nothing');
    expect($$('#helpList .concept')).toHaveLength(0);
    expect($('#helpList [role="status"]')?.textContent).toContain('Nothing matches');
  });

  it('opens from a linked address on the sheet it names, and keeps it across screens', async () => {
    history.replaceState(null, '', '/?topic=trouble.start');
    await mount('#/dashboard');
    expect($('#dock')?.classList.contains('open')).toBe(true);
    expect($('.dock-head b')?.textContent).toBe('An agent won’t start');
    await go('#/network');
    expect($('.dock-head b')?.textContent).toBe('An agent won’t start');
    expect(location.search).toBe('?topic=trouble.start');
  });

  it('closes on Escape', async () => {
    await mount('#/dashboard');
    await click($('#rail [data-dockbtn="help"]'));
    await click($('[data-help-full]'));
    expect($('#dock.open')).not.toBeNull();
    await press('Escape');
    expect($('#dock.open')).toBeNull();
  });

  it('is one thing behind Help and ?: both open this screen\'s numbered notes, and the full help lists no second copy of them', async () => {
    await mount('#/people');
    await click($('#rail [data-dockbtn="help"]'));
    expect($('#helpLayer')).not.toBeNull();
    expect($('#rail [data-dockbtn="help"]')?.classList.contains('on')).toBe(true);
    await click($('#rail [data-dockbtn="help"]'));
    expect($('#helpLayer')).toBeNull();
    await press('?', {}, document.body);
    expect($('#helpLayer')).not.toBeNull();
    await click($('[data-help-full]'));
    expect($$('.dock-in [data-act="explain"]')).toHaveLength(1);
    expect($('.dock-in .help-this-page')).toBeNull();
    expect($('.dock-in')?.textContent).not.toContain('On this page');
  });
});

describe('the help address', () => {
  it('round-trips, keeping every other member', () => {
    const states = [CLOSED_HELP, { page: true, topic: null }, { page: false, topic: 'index' }, { page: true, topic: 'start.cambium' }];
    for (const state of states) {
      const params = withHelpState(new URLSearchParams('x=1'), state);
      expect(helpStateFromParams(params)).toEqual(state);
      expect(params.get('x')).toBe('1');
    }
  });

  it('opens the index for a topic that names nothing', () => {
    expect(helpStateFromParams(new URLSearchParams('topic=no.such'))).toEqual({ page: false, topic: 'index' });
  });
});

describe('the help registry', () => {
  it('has one entry per id, every one in a track, and every related topic resolves', () => {
    expect(new Set(CONCEPTS.map((c) => c.id)).size).toBe(CONCEPTS.length);
    const tracks = new Set(TRACKS.map(([id]) => id));
    for (const c of CONCEPTS) {
      expect(tracks.has(c.track)).toBe(true);
      expect(c.body.length).toBeGreaterThan(0);
      for (const id of c.related) expect(conceptById(id), c.id + ' → ' + id).toBeDefined();
    }
  });

  it('names a real sheet for every page, and every step’s link is a screen the page help knows', () => {
    for (const [key, page] of Object.entries(PAGES)) expect(conceptById(page.topic), key).toBeDefined();
    for (const c of CONCEPTS) for (const step of c.steps) {
      if (!step.href) continue;
      expect(step.href.startsWith('#/'), step.href).toBe(true);
      expect(PAGES[pageKeyOf(step.href.slice(1).split('?')[0])], c.id + ': ' + step.href).toBeDefined();
    }
  });

  it('reaches every screen in the key table', () => {
    const named = goShortcuts().flatMap(([keys]) => keys.split(' or ').map((k) => k.slice(2)));
    expect(named.sort()).toEqual(Object.keys(GO).sort());
  });

  it('keys every page a person can reach', () => {
    expect(pageKeyOf('/')).toBe('dashboard');
    expect(pageKeyOf('/agents/new')).toBe('add-agent');
    expect(pageKeyOf('/people/new')).toBe('add-person');
    expect(pageKeyOf('/people/view/agents')).toBe('people');
    expect(pageKeyOf('/file/A1/access')).toBe('file:access');
    expect(pageKeyOf('/file/A1/budgets')).toBe('file:budgets');
    expect(pageKeyOf('/file/A1/provisioning')).toBe('file:provisioning');
    expect(pageKeyOf('/file/A1/record')).toBe('file:record');
    expect(pageKeyOf('/file/A1/start')).toBe('start');
    expect(pageKeyOf('/team/A1')).toBe('start');
    expect(pageKeyOf('/access/import')).toBe('import');
    expect(pageKeyOf('/resources')).toBe('resources');
    expect(pageKeyOf('/access/drafts')).toBe('drafts');
    expect(pageKeyOf('/access/can')).toBe('access');
    expect(pageKeyOf('/apps/connect')).toBe('apps-connect');
    expect(PAGES['apps-connect'].regions.map((region) => region.title)).toEqual(['The app', 'Where it sends people', 'The permissions it asks for', 'Approve or Decline', 'Known apps']);
    for (const key of ['network', 'apps', 'secrets', 'settings', 'connections', 'me', 'canvas', 'graph', 'requests', 'reviews', 'roles']) expect(PAGES[pageKeyOf('/' + key)], key).toBeDefined();
  });
});
