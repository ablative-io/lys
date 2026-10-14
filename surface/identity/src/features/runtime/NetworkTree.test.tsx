import { act, Profiler } from 'react';
import { createRoot } from 'react-dom/client';
import { afterEach, expect, it, vi } from 'vitest';
import { NetworkTree, NetworkBranches } from './NetworkTree';
import { branches, readNetworkTree } from './network-tree';
import { click, mount, serve, settle } from '../../../tests/harness';
import { ok, refused, SERVICE } from '../../../tests/fixtures';
import { keepOperations, readKeeping } from './canvas-kept';

const machine = (id: string, state = 'in_use') => ({ id, name: id, kind: 'desktop', state, runtime: null, may_run: [{ id: 'agent', display_name: 'Long display name' }] });
const seat = (name: string, state: string) => ({ name, agent: 'agent', machine: 'one', state, transport: 'pty', session: 'session-' + name });
const list = { runner: 'read', seats: ['online-idle', 'online-working', 'offline', 'not-seen', 'unknown'].map((state) => seat(state, state)) };
let root: ReturnType<typeof createRoot>;
afterEach(() => { if (root) act(() => root.unmount()); });
async function render(element: React.ReactNode) {
  const host = document.createElement('div'); document.body.append(host); root = createRoot(host);
  await act(async () => { root.render(element); });
  return host;
}
it('C737 draws every state and empty branch, excludes retired machines and names runner reasons', async () => {
  const value = branches({ machines: [machine('one'), machine('retired', 'retired'), machine('empty')] } as never, { ...list, runner: 'unknown', reason: 'runner_unreachable: no connection' } as never);
  const host = await render(<NetworkBranches branches={value} selected={null} select={vi.fn()} />);
  expect(host.querySelectorAll('[data-machine]')).toHaveLength(2);
  // Every agent a computer may run is under it, once: the one with seats on 'one', the same agent seatless on 'empty'.
  expect(host.querySelectorAll('[data-agent]')).toHaveLength(2);
  expect(host.querySelectorAll('[data-seat]')).toHaveLength(5);
  for (const state of list.seats.map((each) => each.state)) expect(host.querySelector('[data-state="' + state + '"]')).not.toBeNull();
  expect([...host.querySelectorAll('[data-mark]')].map((leaf) => leaf.getAttribute('data-mark'))).toEqual(['●', '◉', '○', '◌', '?', '◌']);
  expect(host.querySelector('[data-machine="empty"] [data-agent-only]')?.querySelector('[title]')?.getAttribute('title')).toContain('no seat on this computer');
  expect(host.textContent).not.toContain('no seats');
  // The agent is named once, on its row; the seats under it carry their own names and the agent's name for a reader.
  expect(host.querySelectorAll('[data-agent] > details > summary[title="Long display name"]')).toHaveLength(1);
  expect(host.querySelector('[data-seat="offline"] a')?.getAttribute('aria-label')).toBe('Long display name, seat offline, pty · offline');
  // A tree like an editor's: names, chevrons and a dot, and nothing else in it. Reasons and Start are the terminal space's; a reason is in the row's title.
  expect(host.querySelectorAll('button, .network-reason, p')).toHaveLength(0);
  expect(host.querySelector('[data-seat="offline"] a')?.getAttribute('title')).toContain('runner_unreachable');
  expect(host.textContent).not.toContain('runner_unreachable');
  // Computers and agents fold: the tree is the browser's own disclosure, open first.
  expect(host.querySelectorAll('details[open]')).toHaveLength(3);
  expect(host.querySelectorAll('[role="treeitem"]')).toHaveLength(9);
});
it('C737 draws the tree before the runners are surveyed, and keeps a survey refusal in the seatless row title, never on the page', async () => {
  const two = { ...machine('one'), may_run: [{ id: 'agent', display_name: 'Long display name' }, { id: 'other', display_name: 'Seatless' }] };
  const routes = { '/network': ok({ machines: [two], next: null }), '/seats': ok({ runner: 'read', seats: [seat('held', 'offline')] }) };
  const tree = () => render(<NetworkTree selected={null} select={vi.fn()} paneWidth={300} resize={vi.fn()} />);
  const row = (host: HTMLElement) => host.querySelector('[data-agent-only="other"]');
  // The survey is refused: the refusal is in the row's title and nowhere in the page's text.
  serve({ ...routes, '/seats/unregistered': refused(503, 'RunnerUnavailable', 'the runner closed the connection') });
  let host = await tree();
  await settle();
  expect(host.querySelectorAll('[data-seat]')).toHaveLength(1);
  expect(row(host)?.querySelector('[title]')?.getAttribute('title')).toContain('RunnerUnavailable: the runner closed the connection');
  expect(host.querySelector('.network-list')?.textContent).not.toContain('RunnerUnavailable');
  expect(host.querySelectorAll('.network-list p')).toHaveLength(0);
  await act(async () => { root.unmount(); });
  // The survey answers: a session held for the seatless agent marks it running; a runner that did not answer is named in the title.
  serve({ ...routes, '/seats/unregistered': ok({ sessions: [{ session: 'op-1', machine: 'one', agent: 'other', pid: 42, started_at: 1 }], unanswered: [{ machine: 'two', name: 'Dean laptop', refusal: 'runner_unreachable', reason: 'not dialled in' }] }) });
  host = await tree();
  await settle();
  expect(row(host)?.getAttribute('data-state')).toBe('online-working');
  expect(row(host)?.querySelector('[title]')?.getAttribute('title')).toContain('pid 42');
  await act(async () => { root.unmount(); });
  // The survey never answers (last, since a read in flight is shared): the tree is drawn all the same, the row unmarked and saying why.
  serve({ ...routes, '/seats/unregistered': (() => new Promise(() => undefined)) as never });
  host = await tree();
  await settle();
  expect(host.querySelectorAll('[data-seat]')).toHaveLength(1);
  expect(row(host)?.getAttribute('data-state')).toBe('not-seen');
  expect(row(host)?.querySelector('[title]')?.getAttribute('title')).toContain('has not answered yet');
  expect(host.querySelectorAll('.network-list p')).toHaveLength(0);
});
it('C737 names one unreadable agent name on the agent row, not on every seat', () => {
  const value = branches({ machines: [{ ...machine('one'), may_run: [] }] } as never, list as never, {}, { agent: 'AgentUnreadable: no display name' });
  expect(value[0].agents).toHaveLength(1);
  expect(value[0].agents[0].reason).toBe('AgentUnreadable: no display name');
  expect(value[0].agents[0].leaves).toHaveLength(5);
  expect(value[0].leaves).toHaveLength(5);
});
it('C737 starts a chosen offline seat from the terminal space with a fresh operation and immediately uses the answered state', async () => {
  const posted: { path: string; body: unknown }[] = [];
  serve({ '/network': ok({ machines: [machine('one')], next: null }), '/seats': ok({ runner: 'read', seats: [seat('stopped', 'offline')] }), 'POST /seats/stopped/start': ok({ seat: seat('stopped', 'online-idle'), session: 'session-stopped' }) }, posted);
  const host = await render(<NetworkTree selected="stopped" select={vi.fn()} paneWidth={300} resize={vi.fn()} />);
  expect(host.querySelector('.network-list button')).toBeNull();
  await act(async () => { (host.querySelector('.network-pane button[aria-label="Start stopped"]') as HTMLElement).click(); });
  expect(posted.find((each) => each.path === '/seats/stopped/start')?.body).toEqual({ operation: expect.stringMatching(/^op-[0-9a-f]{32}$/) });
  expect(host.querySelector('[data-seat="stopped"]')?.getAttribute('data-state')).toBe('online-idle');
});
it('C737 reads all machine pages and refuses malformed or repeated pages', async () => {
  serve({ '/network': ok({ machines: [machine('one')], next: 'page-two' }), '/network?after=page-two': ok({ machines: [machine('two')], next: null }), '/seats': ok(list) });
  expect((await readNetworkTree()).machines).toHaveLength(2);
  serve({ '/network': ok({ machines: [machine('one')], next: 'again' }), '/network?after=again': ok({ machines: [], next: 'again' }), '/seats': ok(list) });
  await expect(readNetworkTree()).rejects.toThrow('PagingRepeated');
  serve({ '/network': ok({ machines: [{ id: 'broken' }] }), '/seats': ok(list) });
  await expect(readNetworkTree()).rejects.toThrow('NetworkUnreadable');
});
it('C737 renders ten machines and two hundred seats within 50 ms of React render work', async () => {
  const machines = Array.from({ length: 10 }, (_, index) => machine(String(index)));
  const seats = Array.from({ length: 200 }, (_, index) => ({ ...seat(String(index), 'online-idle'), machine: String(index % 10) }));
  let duration = 0;
  const host = await render(<Profiler id="tree" onRender={(_id, _phase, elapsed) => { duration = elapsed; }}><NetworkBranches branches={branches({ machines } as never, { runner: 'read', seats } as never)} selected={null} select={vi.fn()} /></Profiler>);
  expect(host.querySelectorAll('[data-seat]')).toHaveLength(200);
  expect(duration).toBeLessThan(50);
});
it('C739 first visits and old layouts open the tree with canvas, sessions and proxy one click away', async () => {
  const routes = { ...SERVICE, '/canvas': ok({ arrangement: { boxes: {}, open: [], groups: [], notes: [], links: [], widgets: [] }, layouts: [] }), '/network': ok({ machines: [machine('one')] }), '/seats': ok(list) };
  const { container } = await mount('#/canvas', routes);
  expect(container.innerHTML).toContain('network-branches');
  expect(container.querySelector('.operations-swap a[href="#/canvas?view=canvas"]')).not.toBeNull();
  expect(container.querySelector('.operations-swap a[href="#/canvas?view=sessions"]')).not.toBeNull();
  expect(container.querySelector('.operations-swap a[href="#/canvas?view=proxy"]')).not.toBeNull();
});
it('C739 restores a service-kept canvas choice, selected leaf and pane width', async () => {
  const operations = { view: 'canvas', selected: 'online-idle', pane_width: 320 };
  const arrangement = { boxes: {}, open: [], groups: [], notes: [], links: [], widgets: [], operations };
  const routes = { ...SERVICE, '/canvas': ok({ arrangement, layouts: [] }), '/runtime/live': ok({ sessions: [], unanswered: [] }) };
  expect((await (serve(routes), readKeeping())).operations).toEqual(operations);
  const { container } = await mount('#/canvas', routes);
  expect(container.querySelector('.operations-swap a[aria-current="page"]')?.textContent).toBe('Canvas');
  expect(container.querySelector('.network-tree')).toBeNull();
});
it('C739 keeps one change with existing canvas marks and names a server refusal without browser storage', async () => {
  const arrangement = { boxes: {}, open: [], groups: [], notes: [{ id: 'note:keep', text: 'keep this', x: 1, y: 1, w: 200, h: 100 }], links: [], widgets: [] };
  const routes = { ...SERVICE, '/canvas': ok({ arrangement, layouts: [] }), '/network': ok({ machines: [machine('one')] }), '/seats': ok(list), 'PUT /canvas': refused(400, 'RequestMalformed', 'unknown field operations') };
  const { container, posted } = await mount('#/canvas', routes);
  await click(container.querySelector('.operations-swap a[href="#/canvas?view=sessions"]'));
  const changes = posted.filter((each) => each.path === 'PUT /canvas');
  expect(changes).toHaveLength(1);
  expect(changes[0].body).toEqual({ arrangement: { ...arrangement, operations: { view: 'sessions', selected: null, pane_width: 300 } } });
  expect(container.textContent).toContain('The Lys service is older than this page');
  expect(container.textContent).toContain('unknown field operations');
  expect(localStorage.getItem('lys.canvas')).toBeNull();
});
it('C739 requires the service to return all three kept fields instead of assuming a save succeeded', async () => {
  const posted: { path: string; body: unknown }[] = [];
  serve({ '/canvas': ok({ arrangement: null }), 'PUT /canvas': ok({ arrangement: { boxes: {}, open: [] } }) }, posted);
  await expect(keepOperations({ view: 'tree', selected: null, pane_width: 300 })).rejects.toMatchObject({ refusal: { refusal: 'OperationsNotKept' } });
  expect(posted).toHaveLength(1);
});
it('C739 the Canvas view draws the canvas with no agent running, and says why it has no windows', async () => {
  const routes = { ...SERVICE, '/canvas': ok({ arrangement: null, layouts: [] }), '/runtime/live': ok({ sessions: [], unanswered: [] }) };
  const { container } = await mount('#/canvas?view=canvas', routes);
  expect(container.querySelector('.session-canvas-scroll')).not.toBeNull();
  expect(container.querySelector('.canvas-side > .running-list')).toBeNull();
  expect(container.textContent).toContain('No agent is running, so the canvas has no windows yet.');
});
it('C739 a tree width outside what the service keeps is not read as kept', async () => {
  const arrangement = { boxes: {}, open: [], groups: [], notes: [], links: [], widgets: [], operations: { view: 'tree', selected: null, pane_width: 720 } };
  serve({ ...SERVICE, '/canvas': ok({ arrangement, layouts: [] }) });
  expect((await readKeeping()).where).toBe('browser');
});
it('C739 an explicit canvas layout or tool-panel address keeps its canvas destination', async () => {
  const routes = { ...SERVICE, '/canvas': ok({ arrangement: null, layouts: [] }), '/runtime/live': ok({ sessions: [], unanswered: [] }) };
  const { container } = await mount('#/canvas?panel=layouts&layout=Morning', routes);
  expect(container.querySelector('.operations-swap a[aria-current="page"]')?.textContent).toBe('Canvas');
  expect(container.querySelector('.canvas-side')).not.toBeNull();
});
