import { act, Profiler } from 'react';
import { createRoot } from 'react-dom/client';
import { afterEach, expect, it, vi } from 'vitest';
import { NetworkTree, NetworkBranches } from './NetworkTree';
import { branches, readNetworkTree } from './network-tree';
import { click, mount, serve } from '../../../tests/harness';
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
  const host = await render(<NetworkBranches branches={value} selected={null} select={vi.fn()} change={vi.fn()} />);
  expect(host.querySelectorAll('[data-machine]')).toHaveLength(2);
  expect(host.querySelectorAll('[data-seat]')).toHaveLength(5);
  for (const state of list.seats.map((each) => each.state)) expect(host.querySelector('[data-state="' + state + '"]')).not.toBeNull();
  expect([...host.querySelectorAll('[data-mark]')].map((leaf) => leaf.getAttribute('data-mark'))).toEqual(['●', '◉', '○', '◌', '?']);
  expect(host.textContent).toContain('no seats');
  expect(host.querySelectorAll('[title="Long display name"]')).toHaveLength(5);
  expect(host.querySelectorAll('[data-seat] .network-reason')).toHaveLength(5);
  expect(host.textContent).toContain('runner_unreachable');
});
it('C737 starts an offline seat with a fresh operation and immediately uses the answered state', async () => {
  const posted: { path: string; body: unknown }[] = [];
  serve({ '/network': ok({ machines: [machine('one')], next: null }), '/seats': ok({ runner: 'read', seats: [seat('stopped', 'offline')] }), 'POST /seats/stopped/start': ok({ seat: seat('stopped', 'online-idle'), session: 'session-stopped' }) }, posted);
  const host = await render(<NetworkTree selected={null} select={vi.fn()} paneWidth={600} resize={vi.fn()} />);
  await act(async () => { (host.querySelector('button[aria-label="Start stopped"]') as HTMLElement).click(); });
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
  const host = await render(<Profiler id="tree" onRender={(_id, _phase, elapsed) => { duration = elapsed; }}><NetworkBranches branches={branches({ machines } as never, { runner: 'read', seats } as never)} selected={null} select={vi.fn()} change={vi.fn()} /></Profiler>);
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
  const operations = { view: 'canvas', selected: 'online-idle', pane_width: 720 };
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
  expect(changes[0].body).toEqual({ arrangement: { ...arrangement, operations: { view: 'sessions', selected: null, pane_width: 600 } } });
  expect(container.textContent).toContain('unknown field operations');
  expect(localStorage.getItem('lys.canvas')).toBeNull();
});
it('C739 requires the service to return all three kept fields instead of assuming a save succeeded', async () => {
  const posted: { path: string; body: unknown }[] = [];
  serve({ '/canvas': ok({ arrangement: null }), 'PUT /canvas': ok({ arrangement: { boxes: {}, open: [] } }) }, posted);
  await expect(keepOperations({ view: 'tree', selected: null, pane_width: 600 })).rejects.toMatchObject({ refusal: { refusal: 'OperationsNotKept' } });
  expect(posted).toHaveLength(1);
});
it('C739 an explicit canvas layout or tool-panel address keeps its canvas destination', async () => {
  const routes = { ...SERVICE, '/canvas': ok({ arrangement: null, layouts: [] }), '/runtime/live': ok({ sessions: [], unanswered: [] }) };
  const { container } = await mount('#/canvas?panel=layouts&layout=Morning', routes);
  expect(container.querySelector('.operations-swap a[aria-current="page"]')?.textContent).toBe('Canvas');
  expect(container.querySelector('.canvas-side')).not.toBeNull();
});
