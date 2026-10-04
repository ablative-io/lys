/**
 * The master off switch on the Dashboard: Stop everything for the administrator only, its question as a row of
 * Running now, the exact body it sends and what it did said by names; while everything is stopped one line across the
 * top and every Start disabled with why; Let agents start again; and a pull with no answer asked about, never sent twice.
 */
import { afterEach, describe, expect, it } from 'vitest';
import { clock } from '../src/features/file/time';
import { $, $$, click, mount, settle, text, type, unmountAll, unreachable } from './harness';
import { ADA, ARCHIVIST, COURIER, CORD, SCRIBE, SERVICE, ok } from './fixtures';
import type { Answer, Route } from './fixtures';

afterEach(() => sessionStorage.clear());

const LAB = 'op-' + 'b'.repeat(32);
const AWAY = 'op-' + 'c'.repeat(32);
const AT = 1790000000;
const PULLED = { operation: 'op-' + '7'.repeat(32), by: ADA, by_name: 'Ada (test person)', reason: 'the building is on fire', kill: false, at: AT };
const stopping = (pulled: typeof PULLED | null, may_pull: boolean) => ok({ ...CORD, pulled, may_pull });
const session = (id: string, agent: string, agent_name: string, machine: string, machine_name: string) => ({ session: id, agent, agent_name, machine, machine_name });
const row = () => $('section[aria-label="Running now"] tr.dash-cord');
const pulls = (posted: { path: string; body: unknown }[]) => posted.filter((entry) => entry.path === '/runtime/stop-everything');

describe('Stop everything', () => {
  it('is offered in the head of Running now to an administrator, and to no one else', async () => {
    await mount('#/', SERVICE);
    expect($('section[aria-label="Running now"] .section-h [data-act="stop-everything"]')?.textContent).toBe('Stop everything');
    expect($('.dash-cord-line')).toBeNull();
    expect(unreachable()).toEqual([]);
    unmountAll();
    await mount('#/', { ...SERVICE, '/runtime/stop-everything': stopping(null, false) });
    expect($('[data-act="stop-everything"]')).toBeNull();
  });

  it('asks in a row of Running now, sends the reason and the kill tick, and says what it did by agent and computer names', async () => {
    const answered = (body: unknown): Answer => {
      const asked = body as { operation: string; reason: string; kill: boolean };
      return ok({
        operation: asked.operation, pulled: { ...PULLED, operation: asked.operation, reason: asked.reason, kill: asked.kill },
        stopped: [session('op-' + '1'.repeat(32), SCRIBE, 'Scribe', LAB, 'Lab'), session('op-' + '2'.repeat(32), COURIER, 'Courier', LAB, 'Lab')],
        still_running: [session('op-' + '3'.repeat(32), ARCHIVIST, 'Archivist', AWAY, 'Away box')],
        unreached: [{ machine: AWAY, machine_name: 'Away box', refusal: 'runner_not_dialled_in', reason: "this computer's runner is not connected to Lys right now" }],
        handles_ended: ['h-1'], handles_refused: [],
      });
    };
    const { posted } = await mount('#/', { ...SERVICE, 'POST /runtime/stop-everything': answered as Route });
    await click($('[data-act="stop-everything"]'));
    expect(row()?.textContent).toContain('This stops every running agent on every computer. Nothing Lys started is left running.');
    expect(pulls(posted)).toEqual([]);
    const now = $$('section[aria-label="Running now"] tr.dash-cord button').find((button) => button.textContent === 'Stop everything now');
    expect(now?.hasAttribute('disabled')).toBe(true);
    await type(row()?.querySelector('input:not([type="checkbox"])') ?? null, 'a runaway loop');
    await click(row()?.querySelector('input[type="checkbox"]') ?? null);
    expect(row()?.textContent).toContain('Kill anything that does not stop');
    await click(now ?? null);

    expect(pulls(posted)).toHaveLength(1);
    const body = pulls(posted)[0].body as { operation: string };
    expect(body).toEqual({ operation: body.operation, reason: 'a runaway loop', kill: true });
    expect(body.operation).toMatch(/^op-[0-9a-f]{32}$/);
    const said = [...(row()?.querySelectorAll('p') ?? [])].map((p) => p.textContent);
    expect(said).toEqual([
      'Stopped 2: Scribe on Lab and Courier on Lab.',
      'Still running: Archivist on Away box.',
      "Away box could not be reached: this computer's runner is not connected to Lys right now.",
    ]);
    expect(row()?.textContent).not.toMatch(/op-|agent-/);
    await click($$('tr.dash-cord button').find((button) => button.textContent === 'Close') ?? null);
    expect(row()).toBeNull();
  });

  it('says once across the top who stopped everything, when and why, and disables every Start with why', async () => {
    await mount('#/', { ...SERVICE, '/runtime/stop-everything': stopping(PULLED, true), ['/agents/' + SCRIBE + '/provisioning']: ok({ profile: { harness: { name: 'Claude Code' } } }) });
    expect($('.dash-cord-line p')?.textContent).toBe('Everything is stopped: the building is on fire, by Ada (test person), ' + clock(AT) + '. No agent can be started.');
    expect(text().split('Everything is stopped')).toHaveLength(2);
    const starts = $$('[data-act="start"]');
    expect(starts.length).toBeGreaterThan(0);
    for (const start of starts) {
      expect(start.hasAttribute('disabled')).toBe(true);
      expect(start.title).toContain('the building is on fire');
    }
    expect($('.dash-cord-line [data-act="let-agents-start"]')?.textContent).toBe('Let agents start again');
  });

  it('does not say everything is stopped when the pull left an agent running or a computer unreached', async () => {
    const last = { operation: PULLED.operation, pulled: PULLED, stopped: [session('s1', SCRIBE, 'Scribe', 'm1', 'Lab')],
      still_running: [session('s2', COURIER, 'Courier', 'm2', 'Away box')],
      unreached: [{ machine: 'm2', machine_name: 'Away box', refusal: 'runner_not_dialled_in', reason: 'not connected' }], handles_ended: [], handles_refused: [] };
    await mount('#/', { ...SERVICE, '/runtime/stop-everything': ok({ ...CORD, pulled: PULLED, last, may_pull: true }) });
    const line = $('.dash-cord-line p')?.textContent ?? '';
    expect(line).not.toContain('Everything is stopped');
    expect(line).toBe('Stop everything was pulled, and not everything has stopped: 1 agent is still running and 1 computer could not be reached. the building is on fire, by Ada (test person), ' + clock(AT) + '. No agent can be started.');
  });

  it('shows a person who is not the administrator the line and no button', async () => {
    await mount('#/', { ...SERVICE, '/runtime/stop-everything': stopping(PULLED, false) });
    expect($('.dash-cord-line p')?.textContent).toContain('Everything is stopped: the building is on fire');
    expect($('[data-act="let-agents-start"]')).toBeNull();
    expect($('[data-act="stop-everything"]')).toBeNull();
  });

  it('lets agents start again under a new operation, and the page reads again', async () => {
    const released = (body: unknown): Answer => {
      const asked = body as { operation: string };
      return ok({ ...CORD, released: { operation: asked.operation, pull: PULLED.operation, by: ADA, by_name: 'Ada (test person)', at: AT + 60 } });
    };
    const { posted, requests } = await mount('#/', { ...SERVICE, '/runtime/stop-everything': stopping(PULLED, true), 'POST /runtime/stop-everything/release': released as Route });
    const reads = requests.filter((path) => path === '/runtime/stop-everything').length;
    await click($('[data-act="let-agents-start"]'));
    const sent = posted.filter((entry) => entry.path === '/runtime/stop-everything/release');
    expect(sent).toHaveLength(1);
    expect((sent[0].body as { operation: string }).operation).toMatch(/^op-[0-9a-f]{32}$/);
    expect(Object.keys(sent[0].body as object)).toEqual(['operation']);
    await settle();
    expect(requests.filter((path) => path === '/runtime/stop-everything').length).toBeGreaterThan(reads);
  });

  it('asks about a pull that had no answer, and sends it again only as it was', async () => {
    const fault: Route = () => ({ status: 500, body: { refusal: 'Unanswered', reason: 'the service did not answer' } });
    const first = await mount('#/', { ...SERVICE, 'POST /runtime/stop-everything': fault });
    await click($('[data-act="stop-everything"]'));
    await type(row()?.querySelector('input:not([type="checkbox"])') ?? null, 'a runaway loop');
    await click($$('tr.dash-cord button').find((button) => button.textContent === 'Stop everything now') ?? null);
    expect(pulls(first.posted)).toHaveLength(1);
    expect(row()?.textContent).toContain('This change has no confirmed answer.');
    unmountAll();

    const again = await mount('#/', SERVICE);
    expect(row()?.textContent).toContain('This change has no confirmed answer.');
    expect($$('tr.dash-cord button').find((button) => button.textContent === 'Stop everything now')?.hasAttribute('disabled')).toBe(true);
    await click($$('tr.dash-cord button').find((button) => button.textContent === 'Check whether Lys saved it') ?? null);
    expect(pulls(again.posted).map((entry) => entry.body)).toEqual(pulls(first.posted).map((entry) => entry.body));
    expect(row()?.textContent).toContain('Nothing was running, so nothing needed stopping.');
  });
});
