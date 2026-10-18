import { readFileSync } from 'node:fs';
import { describe, expect, it } from 'vitest';
import { windowText } from '../src/features/grants/model';
import { breakable } from '../src/features/grants/GrantTable';
import { renderToStaticMarkup } from 'react-dom/server';
import { $, $$, click, mount, text } from './harness';
import { ADA, GRANTS, MODEL, SCRIBE, SERVICE, ok } from './fixtures';

const css = (file: string) => readFileSync(new URL('../src/' + file, import.meta.url), 'utf8');
describe('Surface words and controls', () => {
  it('names an absent start without an epoch and preserves a real start', () => {
    expect(windowText({ ...GRANTS[0], window: { starts_at: 0, ends_at: null }, effective_ends_at: null })).toBe('no start, no end');
    expect(windowText(GRANTS[0])).not.toContain('no start');
  });
  it.each(['#/spelling-mistake', '#/settings/profiles', '#/access/unknown', '#/people/view/unknown'])('names an unknown address %s', async (address) => {
    await mount(address, SERVICE);
    expect($('.page h1')?.textContent).toBe('Not found');
    expect(text()).toContain(address.slice(1));
    // The next step for an address with no page: Home, and Search for it by name.
    expect($('.page a.btn[href="#/"]')?.textContent).toBe('Go Home');
    expect($('.page [data-symbol="search"]')?.getAttribute('aria-label')).toBe('Search for the page by its name');
  });
  it('gives an unknown agent name the agents list instead of an access accusation', async () => {
    await mount('#/team/Unknown%20helper', SERVICE);
    expect(text()).toContain('No agent called Unknown helper');
    expect(text()).not.toContain('PersonNotVisible');
    expect($('a[href="#/people/view/agents"]')).not.toBeNull();
  });
  it('does not offer Stop for a known offline seat and preserves the full identifiers in titles', async () => {
    await mount('#/canvas?view=sessions', {
      ...SERVICE,
      '/seats': ok({ runner: 'read', seats: [{ name: 'Ward helper', agent: SCRIBE, machine: 'op-' + 'e'.repeat(32), state: 'offline', harness: 'Care', profile_version: 1, working_folder: null, responsible: ADA, session: 'op-' + 'f'.repeat(32), created_by: ADA, created_at: 1, revision: 1 }] }),
      '/seats/unregistered': ok({ sessions: [] }),
    });
    const row = $('tr[data-seat="Ward helper"]');
    expect(row?.querySelector('button[aria-label="Start Ward helper"]')).not.toBeNull();
    expect(row?.querySelector('button[aria-label="Stop Ward helper"]')).toBeNull();
    expect(row?.querySelector('[title="' + SCRIBE + '"]')?.textContent).toBe('Scribe');
  });
  it('matches the directory visibility on Graph and keeps retired records reachable on request', async () => {
    await mount('#/graph', SERVICE);
    expect($$('g.gn').map((node) => node.getAttribute('aria-label'))).not.toContain('Lamplighter');
    const toggle = $$('button').find((button) => button.textContent?.startsWith('Show retired'));
    await click(toggle ?? null);
    expect($$('g.gn').map((node) => node.getAttribute('aria-label'))).toContain('Lamplighter');
  });
  it('keeps the count below the Access icon and applies the grant word-boundary rules', async () => {
    expect(css('styles/overlays.css')).not.toMatch(/\.rail:not\(\.open\) \.cnt[^}]*position: absolute/);
    expect(css('features/runtime/session-canvas.css')).toMatch(/operations-swap[^}]*position: static/);
    expect(css('styles/dock.css')).not.toMatch(/\.legend[^}]*position: absolute/);
    const style = document.createElement('style');
    style.textContent = css('features/grants/grants.css') + '\n' + css('styles/screen.css');
    document.head.appendChild(style);
    try {
      const actions = ['read', 'write', 'share'];
      await mount('#/me?tab=account', { ...SERVICE, '/grants': ok({ grants: [{ ...GRANTS[0], actions }], revision: 7 }), '/grants/model': ok({ ...MODEL, action_sentences: { ...MODEL.action_sentences, read: 'Read records', write: 'Write records', share: 'Share records' } }) });
      for (const column of ['Grant', 'Relation']) {
        expect(getComputedStyle($(`td[data-col="${column}"]`) as HTMLElement).whiteSpace).toBe('nowrap');
      }
      const allows = $('td[data-col="Allows"]') as HTMLElement;
      expect(allows.querySelectorAll('li')).toHaveLength(3);
      expect(getComputedStyle(allows).whiteSpace).toBe('normal');
      expect(getComputedStyle(allows.querySelector('ul') as HTMLElement).flexDirection).toBe('column');
      for (const cell of $$('table.grant-cols td, .grant-cols td')) {
        const rules = getComputedStyle(cell);
        expect(rules.overflowWrap).toBe('normal');
        expect(rules.wordBreak).toBe('normal');
        expect(rules.hyphens).toBe('none');
      }
      const reference = 'project:alpha.beta_gamma-delta/epsilon';
      const page = new DOMParser().parseFromString(renderToStaticMarkup(<span>{breakable(reference)}</span>), 'text/html');
      expect(page.body.textContent).toBe(reference);
      expect([...page.querySelectorAll('wbr')].map((br) => br.previousSibling?.textContent)).toEqual(['project:', 'alpha.', 'beta_', 'gamma-', 'delta/']);
      expect(renderToStaticMarkup(<span>{breakable('unbrokenword')}</span>)).not.toContain('<wbr');
    } finally {
      style.remove();
    }
  });
});
