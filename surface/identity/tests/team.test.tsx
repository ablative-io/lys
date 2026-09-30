/** The team screen reads the real teams, people and live sessions into a tree of names, opens the first running agent by itself, starts a stopped one in place, and names a refusal. */
import { describe, expect, it, vi } from 'vitest';
import { $, $$, click, mount, text, unreachable } from './harness';
import { ADA, ARCHIVIST, COURIER, REVIEWER, SCRIBE, SERVICE, ok, refused } from './fixtures';
import { mockTerminal } from './terminal-double';
vi.mock('@gespenst/core', () => ({ createTerminal: mockTerminal }));

const LIVE = 'op-' + '5'.repeat(32);
const LAB = 'op-' + 'b'.repeat(32);
const SHED = 'op-' + 'c'.repeat(32);

const running = { session: LIVE, agent: SCRIBE, machine: LAB, machine_name: 'Lab', runtime: 'lys-runner', shown: 'running', last_reported: 'running', first_report_at: 1790000000, last_report_at: 1790000001, what: 'started', stopped: null, reported_by: 'the runner' };
const machine = (id: string, name: string, runtime: string | null, mayRun: string[]) => ({ id, name, kind: 'laptop', runtime, slots: 1, may_run: mayRun.map((agent) => ({ id: agent, display_name: 'x', state: 'active' })), may_reach: [], named_by: ADA, named_at: 1790000000, state: 'in_use', retired_at: null, last_report_at: null });
const team = (id: string, owner: string, name: string, members: string[]) => ({ id, owner, name, description: '', members, state: 'active', created_by: { provider: 'p', subject: 's' }, created_at: 1790000000, retired_at: null });
const receipt = (answer: unknown) => ok({ session: LIVE, answer, receipt: { index: 1 } });

const routes = {
  ...SERVICE,
  '/teams': ok({ teams: [team('team-1', ADA, 'Ada team', [REVIEWER]), team('team-2', SCRIBE, 'Crew', [COURIER, ARCHIVIST])] }),
  '/runtime/live': ok({ sessions: [running], unanswered: [] }),
  '/network': ok({ machines: [machine(LAB, 'Lab', 'lys-runner', [COURIER]), machine(SHED, 'Shed', null, [])], reports_served: true }),
  ['/agents/' + COURIER + '/runtime/sessions']: ok({ sessions: [{ ...running, session: 'op-' + '7'.repeat(32), agent: COURIER, machine: SHED, machine_name: 'Shed', shown: 'stopped', stopped: { at: 1790000002, confirmation: 'ended' } }] }),
  ['POST /runtime/sessions/' + LIVE + '/resize']: receipt({ kind: 'resized' }),
  ['POST /runtime/sessions/' + LIVE + '/read-bytes']: refused(502, 'runner_unreachable', 'the read was closed'),
};

describe('Team', () => {
  it('draws the tree of names and opens the first running agent by itself', async () => {
    await mount('#/team', routes);
    expect(location.hash).toBe('#/team/' + SCRIBE);
    const names = $$('.tree-name').map((el) => el.textContent);
    expect(names).toEqual(['Reviewer', 'Scribe', 'Courier', 'Archivist']);
    expect($$('.tree-team').map((el) => el.textContent)).toEqual(['Ada team', 'Crew']);
    expect(text()).not.toContain('Ada (test person)');
    expect($('.tree-name[aria-current="page"]')?.textContent).toBe('Scribe');
    expect($('.team-foot')?.textContent).toContain('Lab');
    expect($('form[aria-label="Type to the session"]')).toBeNull();
    expect(unreachable()).toEqual([]);
  });

  it('folds a team away and back, and hides the tree from the foot line', async () => {
    localStorage.clear();
    await mount('#/team/' + SCRIBE, routes);
    await click($$('.tree-team').find((el) => el.textContent === 'Crew') ?? null);
    expect($$('.tree-name').map((el) => el.textContent)).toEqual(['Reviewer', 'Scribe']);
    expect($$('.tree-team').map((el) => el.textContent)).toEqual(['Ada team', 'Crew2']);
    expect(localStorage.getItem('iam.team-folded')).toBe('team-2');
    await click($$('.tree-team').find((el) => el.textContent === 'Crew2') ?? null);
    expect($$('.tree-name').map((el) => el.textContent)).toEqual(['Reviewer', 'Scribe', 'Courier', 'Archivist']);
    expect($('.team-screen')?.getAttribute('data-tree')).toBe('shown');
    await click($('.team-foot-tree'));
    expect($('.team-screen')?.getAttribute('data-tree')).toBe('hidden');
    expect(localStorage.getItem('iam.team-tree')).toBe('hidden');
    await click($('.team-foot-tree'));
    expect($('.team-screen')?.getAttribute('data-tree')).toBe('shown');
    localStorage.clear();
  });

  it('starts a stopped agent in place, the machine it ran on first, and hands over the command when that machine has no runner', async () => {
    const { posted } = await mount('#/team/' + COURIER, { ...routes, ['POST /agents/' + COURIER + '/start-command']: ok({ agent: COURIER, machine: SHED, command: 'lys start courier', executed: false, left_out: [] }) });
    expect(text()).toContain('Courier is not running.');
    const picked = $$('input[name="machine"]').map((el) => [(el as HTMLInputElement).value, (el as HTMLInputElement).checked]);
    expect(picked).toEqual([[SHED, true], [LAB, false]]);
    expect(text()).toContain('ran here last');
    expect(text()).toContain('not yet permitted to run Courier');
    await click($$('button').find((el) => el.textContent === 'Start') ?? null);
    expect(posted).toHaveLength(1);
    expect(posted[0]).toMatchObject({ path: '/agents/' + COURIER + '/start-command', body: { machine: SHED } });
    expect(text()).toContain('has no runner');
    expect(($('textarea.team-command') as HTMLTextAreaElement).value).toBe('lys start courier');
  });

  it('names the refusal when the caller is not signed in', async () => {
    await mount('#/team', { ...routes, '/teams': refused(401, 'NotSignedIn', 'sign in first') });
    expect(text()).toContain('not signed in');
  });
});
