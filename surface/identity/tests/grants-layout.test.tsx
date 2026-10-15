import { readFileSync } from 'node:fs';
import { act } from 'react';
import { createRoot } from 'react-dom/client';
import type { Root } from 'react-dom/client';
import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import { You } from '../src/features/me/You';
import { App } from '../src/App';
import { breakable } from '../src/features/grants/GrantTable';
import type { SessionView } from '../src/features/sessions/Sessions';
import { GRANTS, ME, MODEL, SERVICE, ok } from './fixtures';
import { serve } from './harness';

const actions = Array.from({ length: 25 }, (_, index) => 'action-' + index);
const sentences = actions.map((_, index) => 'Allow operation ' + index + ' for this agent and its registered certificates');
const grant = { ...GRANTS[0], actions };
const sessions: SessionView[] = [true, false].map((current, index) => ({
  id: 'session-' + index,
  login: { issuer: 'https://a-long-sign-in-provider.example.test', subject: 'test-person' },
  started_at: 1790000000,
  ends_at: 1790003600,
  current,
}));
const routes = {
  ...SERVICE,
  '/grants': ok({ grants: [grant], revision: 7 }),
  '/grants/model': ok({
    ...MODEL,
    action_sentences: { ...MODEL.action_sentences, ...Object.fromEntries(actions.map((action, index) => [action, sentences[index]])) },
  }),
  '/sessions': ok({ person: ME.person.id, sessions }),
};

let root: Root;
let style: HTMLStyleElement;

beforeEach(() => {
  style = document.createElement('style');
  // The entry point loads feature styles before the shared screen styles.
  style.textContent = ['features/grants/grants.css', 'features/me/you.css', 'styles/screen.css']
    .map((path) => readFileSync(new URL('../src/' + path, import.meta.url), 'utf8'))
    .join('\n');
  document.head.appendChild(style);
  const container = document.createElement('div');
  document.body.appendChild(container);
  root = createRoot(container);
  serve(routes);
});

afterEach(() => {
  act(() => root.unmount());
  style.remove();
});

function element(selector: string): HTMLElement {
  const found = document.querySelector<HTMLElement>(selector);
  if (!found) throw new Error('Missing layout subject: ' + selector);
  return found;
}

async function showYou(): Promise<void> {
  await act(async () => root.render(<You />));
  expect(document.querySelectorAll('.you-sessions tbody tr')).toHaveLength(2);
}

function expectWordBoundaries(): void {
  const cells = document.querySelectorAll<HTMLElement>('.grant-cols td');
  expect(cells.length).toBeGreaterThan(0);
  for (const cell of cells) {
    const css = getComputedStyle(cell);
    expect(css.overflowWrap, cell.dataset.col).toBe('normal');
    expect(css.wordBreak, cell.dataset.col).toBe('normal');
  }
  for (const column of ['Grant', 'Relation']) {
    expect(getComputedStyle(element(`td[data-col="${column}"]`)).whiteSpace, column).toBe('nowrap');
  }
  expect(getComputedStyle(element('.g-tight')).whiteSpace).toBe('nowrap');
  expect(getComputedStyle(element('.chain .pill')).overflowWrap).toBe('normal');
}

describe('LYSWALK layout contracts', () => {
  it('W1 wraps all 25 actions and grant facts on Access and You', async () => {
    location.hash = '#/access';
    await act(async () => root.render(<App />));
    expect(element('td[data-col="Allows"]').textContent).toContain(sentences[24]);
    expect(getComputedStyle(element('td[data-col="Allows"]')).whiteSpace).not.toBe('nowrap');
    expect(element('td[data-col="Allows"]').querySelectorAll('li')).toHaveLength(25);
    expect(getComputedStyle(element('td[data-col="Window"]')).whiteSpace).not.toBe('nowrap');
    expect(getComputedStyle(element('.grant-cols table')).width).toBe('100%');
    expect(getComputedStyle(element('.grant-cols th')).overflowWrap).toBe('normal');
    expectWordBoundaries();
    await showYou();
    expectWordBoundaries();
    for (const column of ['Allows', 'Window', 'Last used', 'May pass on']) {
      expect(getComputedStyle(element(`td[data-col="${column}"]`)).whiteSpace, column).not.toBe('nowrap');
    }
    expect(getComputedStyle(element('.g-change')).flexWrap).toBe('wrap');
  });

  it('breaks references only after their own dot colon underscore hyphen and slash', async () => {
    const reference = 'project:alpha.beta_gamma-delta/epsilon';
    await act(async () => root.render(<span data-reference>{breakable(reference)}</span>));
    const shown = element('[data-reference]');
    expect(shown.textContent).toBe(reference);
    const breaks = [...shown.querySelectorAll('wbr')];
    expect(breaks).toHaveLength(5);
    expect(breaks.map((br) => br.previousSibling?.textContent)).toEqual(['project:', 'alpha.', 'beta_', 'gamma-', 'delta/']);
    expect(breaks.every((br) => /[.:_/-]$/.test(br.previousSibling?.textContent ?? ''))).toBe(true);
    await act(async () => root.render(<span data-reference>{breakable('unbrokenword')}</span>));
    expect(element('[data-reference]').querySelectorAll('wbr')).toHaveLength(0);
  });

  it('W2 fits the sessions table and keeps both end-session controls visible', async () => {
    await showYou();
    expect(getComputedStyle(element('.you-sessions table')).width).toBe('100%');
    expect(getComputedStyle(element('.you-sessions td:first-child')).minWidth).not.toBe('170px');
    for (const button of document.querySelectorAll<HTMLElement>('.you-sessions td button')) {
      expect(button.getAttribute('aria-label')).toMatch(/^(Sign out|End session)$/);
      expect(getComputedStyle(button.querySelector('.act-word') as HTMLElement).display).toBe('none');
      expect(button.querySelector('svg[aria-hidden="true"]')).not.toBeNull();
      for (let parent = button.parentElement; parent && !parent.classList.contains('you-account'); parent = parent.parentElement) {
        const css = getComputedStyle(parent);
        expect([css.overflow, css.overflowX], parent.className || parent.tagName).not.toEqual(expect.arrayContaining(['hidden']));
      }
    }
    for (const cell of document.querySelectorAll<HTMLElement>('.you-sessions td')) {
      expect(getComputedStyle(cell).whiteSpace).not.toBe('nowrap');
      expect(getComputedStyle(cell).overflowWrap).toBe('normal');
      expect(getComputedStyle(cell).wordBreak).not.toBe('break-all');
    }
    expect(getComputedStyle(element('.you-sessions td .note')).overflowWrap).toBe('normal');
  });

  it('W3 sizes every account card to its content without a private scrollbar', async () => {
    await showYou();
    const cards = document.querySelectorAll<HTMLElement>('.you-account-row > .card');
    expect(cards).toHaveLength(4);
    expect(getComputedStyle(element('.you-account-row')).alignItems).toBe('start');
    for (const card of cards) {
      const css = getComputedStyle(card);
      expect(css.overflow, card.id || card.className).toBe('visible');
      expect(css.maxHeight, card.id || card.className).not.toBe('360px');
    }
    expect(getComputedStyle(element('.you-sessions-scroll')).overflow).toBe('visible');
  });
});
