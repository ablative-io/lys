/** The one permissions matrix shows every action any relation carries, and names are told apart where two are the same. */
import { renderToStaticMarkup } from 'react-dom/server';
import { describe, expect, it } from 'vitest';
import { SchemaMatrix } from '../src/features/apps/SchemaMatrix';
import { nameOf, whoTitle } from '../src/features/grants/model';
import type { GrantWorld } from '../src/features/grants/model';

const table = (html: string): Document => new DOMParser().parseFromString(html, 'text/html');
const rows = (page: Document) => [...page.querySelectorAll('tbody tr')].map((row) => [...row.children].map((cell) => cell.textContent));

describe('The permissions matrix', () => {
  it('gives a row to an action a relation carries that its kind did not list, after the listed ones', () => {
    const page = table(renderToStaticMarkup(<SchemaMatrix label="Its schema" kinds={[['channel', { actions: ['read'], relations: { reader: ['read'], poster: ['read', 'write'] } }]]} />));
    expect([...page.querySelectorAll('thead th')].map((cell) => cell.textContent)).toEqual(['channel', 'reader', 'poster']);
    expect(rows(page)).toEqual([['read', '✓', '✓'], ['write', '', '✓']]);
  });
  it('names a folded one-action relation beside an action its kind did not list', () => {
    const relations: Record<string, string[]> = { owner: ['read', 'write'] };
    for (let index = 0; index < 8; index += 1) relations['only.act' + index] = ['act' + index];
    const page = table(renderToStaticMarkup(<SchemaMatrix label="Its schema" kinds={[['thing', { actions: ['read', 'write'], relations }]]} />));
    const shown = rows(page);
    expect(shown.map((row) => row[0])).toEqual(['read', 'write', 'act0', 'act1', 'act2', 'act3', 'act4', 'act5', 'act6', 'act7']);
    expect(shown[2]).toEqual(['act0', '', 'only.act0']);
    expect(shown[9]).toEqual(['act7', '', 'only.act7']);
    // Every relation of the schema is on the screen, as a column or beside its action.
    for (const relation of Object.keys(relations)) expect(page.body.textContent).toContain(relation);
  });
});

describe('A name that two hold', () => {
  it('is told apart in its title by whose agent it is and its file number', () => {
    const ada = 'person-' + 'a'.repeat(32), bea = 'person-' + 'b'.repeat(32);
    const one = 'agent-' + '1'.repeat(32), two = 'agent-' + '2'.repeat(32), account = 'service-' + '3'.repeat(32);
    const w = { who: new Map([
      [ada, { name: 'Ada', state: 'active', kind: 'person', responsible: null }],
      [bea, { name: 'Bea', state: 'active', kind: 'person', responsible: null }],
      [one, { name: 'Scout', state: 'active', kind: 'agent', responsible: ada }],
      [two, { name: 'Scout', state: 'active', kind: 'agent', responsible: bea }],
      [account, { name: 'Loader', state: 'active', kind: 'service_account', responsible: ada }],
    ]) } as unknown as GrantWorld;
    expect(nameOf(w, one)).toBe(nameOf(w, two));
    expect(whoTitle(w, one)).toBe('Scout, agent of Ada, A/11111111');
    expect(whoTitle(w, two)).toBe('Scout, agent of Bea, A/22222222');
    expect(whoTitle(w, ada)).toBe('Ada, P/aaaaaaaa');
    expect(whoTitle(w, account)).toBe('Loader, account of Ada, S/33333333');
    // Someone outside the caller's view has no name to show; the file number alone stands for them.
    expect(whoTitle(w, 'agent-' + '9'.repeat(32))).toBe('A/99999999');
  });
});
