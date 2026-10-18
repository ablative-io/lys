/**
 * The places of the shell: the rail is CONSENSUS.md's of 18 October 2026 (it replaced the 4 October walk's Network
 * and the graph's own rail entry), one permissions matrix serves Apps and Model, every page head has its title before its tabs, a
 * count of one is said in the singular, and an empty list says so once.
 */
import { describe, expect, it } from 'vitest';
import { $, $$, click, mount, press, text, type, unmountAll } from './harness';
import { MODEL, SERVICE, ok } from './fixtures';
import { counted, money, singular } from '../src/shell/count';
import type { ReactNode } from 'react';
import { renderToStaticMarkup } from 'react-dom/server';
import { Listing } from '../src/shell/Listing';

const settings = { source: 'startup_configuration', mutable_in_browser: false, sign_in: { provider_origin: 'https://login.test', session_seconds: 3600, secure_cookie: true }, directory: { roles_configured: true }, permissions: { model_version: 8, projection: 'local' }, secrets: { configured: true }, runtimes: { machines_configured: true, provisioning_configured: true }, storage: { directory_format: 'signed_leaf_log', grant_format: 'signed_leaf_log', requests_configured: true } };
const lys = { id: 'lys', name: 'Lys', state: 'approved', redirects: [], schema: { relations: MODEL.relations }, version: 2, versions: [2], pending: null, client_id: null, service_account: null, client_credentials: [], registered_by: { kind: 'start' }, registered_at: 1 };
const resources = (list: { kind: string; id: string; standing: number; ended: number; holders: number }[]) => ok({ kinds: [...new Set(list.map((entry) => entry.kind))], revision: 1, judged_at: 1790540000, resources: list });
const ROUTES = {
  ...SERVICE,
  '/configuration': ok(settings),
  '/apps': ok({ apps: [lys] }),
  '/connections': ok({ connections: [], health_checked: false }),
  '/sign-in-providers': ok({ providers: [], offered: ['google'], redirect_address: 'http://localhost:8490/auth/v1/providers/callback' }),
  '/resources': resources([]),
};
const before = (a: Element | null, b: Element | null) => Boolean(a && b && a.compareDocumentPosition(b) & Node.DOCUMENT_POSITION_FOLLOWING);

describe('the rail and the palette', () => {
  it('names the computers Computers, gives Apps its own entry, and keeps Roles and the access map off the rail', async () => {
    await mount('#/people', ROUTES);
    const navs = $$('#rail a[data-nav]').map((entry) => entry.dataset.nav);
    expect(navs).toEqual(['dashboard', 'waiting', 'people', 'canvas', 'access', 'network', 'apps', 'secrets', 'settings', 'me']);
    expect($$('#rail a[data-nav] .lbl:not(.kbd)').map((entry) => entry.textContent)).toEqual(['Dashboard', 'Waiting on you', 'People and agents', 'Running agents', 'Access', 'Computers', 'Apps', 'Secrets', 'Settings', 'You']);
    expect($('#rail')?.textContent).not.toMatch(/Network|Operations|Graph|Command|Collapse/);
  });
  it('lights Access on the access map and on Roles', async () => {
    await mount('#/graph', ROUTES);
    expect($('#rail a.on')?.dataset.nav).toBe('access');
    unmountAll();
    await mount('#/roles', ROUTES);
    expect($('#rail a.on')?.dataset.nav).toBe('access');
  });
  it('shows a key, never an address, beside each place in the palette, and words its acts plainly', async () => {
    await mount('#/people', ROUTES);
    await press('k', { metaKey: true }, document.body);
    expect($$('#palList .it[data-to]').map((row) => row.firstElementChild?.textContent)).toEqual(expect.arrayContaining(['Computers', 'Running agents', 'Waiting on you']));
    await type($('#palIn'), 'Access map');
    expect($$('#palList .it[data-to]').map((row) => row.firstElementChild?.textContent)).toEqual(['Access map']);
    await type($('#palIn'), '');
    const rows = $$('#palList .it[data-to]');
    expect(rows.map((row) => row.firstElementChild?.textContent)).not.toEqual(expect.arrayContaining(['Network']));
    for (const row of rows) expect(row.textContent).not.toContain('#/');
    expect(rows.find((row) => row.dataset.to === '#/people')?.lastElementChild?.textContent).toBe('g p');
    const acts = $$('#palList .it[data-n]').map((row) => row.firstElementChild?.textContent);
    expect(acts).toEqual(expect.arrayContaining(['Move the menu and Help to the other side', 'Show or hide menu labels']));
    expect(acts.join(' ')).not.toContain('Toggle');
  });
});

describe('count sentences', () => {
  it('says one in the singular', () => {
    expect(counted(1, 'computers')).toBe('1 computer');
    expect(counted(2, 'computers')).toBe('2 computers');
    expect(counted(1, 'people')).toBe('1 person');
    expect(counted(1, 'people active')).toBe('1 person active');
    expect(counted(1, 'people and agents')).toBe('1 person or agent');
    expect(counted(1500, 'secrets')).toBe('1,500 secrets');
    expect(singular('running sessions')).toBe('running session');
  });
  it('says money one way: US dollars, two decimals, grouped', () => {
    expect([money(0), money(1.5), money(1500), money(0.005)]).toEqual(['US$0.00', 'US$1.50', 'US$1,500.00', 'US$0.01']);
  });
  it('counts and searches one row in the singular', async () => {
    await mount('#/resources', { ...ROUTES, '/resources': resources([{ kind: 'project', id: 'identity', standing: 1, ended: 0, holders: 1 }]) });
    expect($('.tools .count')?.textContent).toBe('1 resource');
    expect($('input[aria-label="Search resources"]')?.getAttribute('placeholder')).toBe('Search 1 resource');
  });
  it('says an empty list is empty once, in the table body', async () => {
    await mount('#/resources', ROUTES);
    expect(text().split('No resources appear in your visible grants yet.')).toHaveLength(2);
    expect($('.listing tbody tr.empty td')?.textContent).toBe('No resources appear in your visible grants yet.');
    expect($('.tools .count')).toBeNull();
    expect(text()).not.toContain('Nothing here yet');
    expect(text()).not.toContain('0 resources');
  });
});

describe('page heads', () => {
  it.each(['#/model', '#/resources', '#/settings', '#/connections'])('%s has its title before its tabs', async (hash) => {
    await mount(hash, ROUTES);
    expect($('#screen nav.tabs')).not.toBeNull();
    expect(before($('#screen h1'), $('#screen nav.tabs'))).toBe(true);
  });
  it('Configuration repeats no rail entry as a link', async () => {
    await mount('#/settings', ROUTES);
    for (const href of ['#/roles', '#/model', '#/access', '#/requests', '#/secrets', '#/network', '#/people']) expect($('#screen a[href="' + href + '"]'), href).toBeNull();
    expect(text()).toContain('Role records');
  });
});

describe('the one permissions matrix', () => {
  it('is the same table on Configuration > Apps for Lys and on Access > Model', async () => {
    const rowsOf = () => $$('table.schema-matrix tbody th[scope="row"]').map((cell) => cell.textContent);
    const columnsOf = () => $$('table.schema-matrix thead th').slice(1).map((cell) => cell.textContent);
    await mount('#/apps', ROUTES);
    await click($('.app-picker .app-choice'));
    const apps = { rows: rowsOf(), columns: columnsOf() };
    unmountAll();
    await mount('#/model', ROUTES);
    expect({ rows: rowsOf(), columns: columnsOf() }).toEqual(apps);
    expect(apps.rows).toEqual(['Edit this resource', 'View this resource', 'Give access to this resource']);
    expect(apps.columns).toEqual(['editor', 'owner', 'viewer']);
  });
  it('names many single-action relations beside their action so the matrix fits the width', async () => {
    const relations: Record<string, string[]> = { editor: ['edit', 'view', 'grant'] };
    for (const action of ['edit', 'view', 'grant']) for (const n of [1, 2, 3]) relations['only.' + action + '.' + n] = [action];
    await mount('#/model', { ...ROUTES, '/grants/model': ok({ ...MODEL, relations }) });
    expect($$('table.schema-matrix thead th').map((cell) => cell.textContent)).toEqual(['What it may do', 'editor', 'The relation for this alone']);
    expect($$('table.schema-matrix tbody td.alone').map((cell) => cell.textContent)).toEqual(['only.edit.1, only.edit.2, only.edit.3', 'only.view.1, only.view.2, only.view.3', 'only.grant.1, only.grant.2, only.grant.3']);
  });
});

describe('the graph', () => {
  it('says what it holds before anything is chosen', async () => {
    await mount('#/graph', ROUTES);
    expect($('.node-card h2')?.textContent).toBe('In this drawing');
    expect($('.node-card')?.textContent).toMatch(/People\d+ (person|people)/);
    expect(text()).not.toContain('Click anything');
  });
});

describe('the one list', () => {
  const list = (items: string[], foot?: ReactNode) => renderToStaticMarkup(<Listing<string> groups={[{ id: '', name: '', depth: 0, lead: null, items, within: items }]}
    columns={[{ head: 'Name', cell: (item) => item }, { head: 'Change', cell: () => null }]} id={(item) => item} href={(item) => '#/' + item}
    words={(item) => item} noun="things" holds={(held) => counted(held.length, 'things')} selected={null} select={() => undefined} foot={foot} />);
  const row = <tr data-add="thing"><td>new</td><td>Add</td></tr>;

  it('keeps its foot as the table\'s last element, after the rows', () => {
    const html = list(['one', 'two'], row);
    expect(html).toMatch(/<\/tbody><tfoot><tr data-add="thing"><td>new<\/td><td>Add<\/td><\/tr><\/tfoot><\/table>/);
  });

  it('shows its foot when the list is empty, after the one sentence saying so', () => {
    const html = list([], row);
    expect(html).toMatch(/<tr class="empty">.*<\/tbody><tfoot><tr data-add="thing">.*<\/tfoot><\/table>/);
    expect(html.split('Nothing here yet.')).toHaveLength(2);
  });

  it('draws no foot when it is given none', () => {
    expect(list(['one'])).not.toContain('<tfoot');
  });
});
