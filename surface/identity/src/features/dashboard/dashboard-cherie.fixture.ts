/**
 * Cherie's dashboard, for looking at the page signed out (`#/dashboard?option=A&fixture=cherie` in a dev build). She is
 * a rung-1 person who set up her first MacBook this weekend on a Claude subscription: Helper runs on it with a US$5.00
 * weekly limit, Researcher is set up and stopped with one goal missed, Sketcher has no program chosen yet. Helper's
 * account reported plan windows, so the page says the plan is what she pays. Nothing here is read from the service.
 */
import type { MeView } from '../../generated';
import type { RuntimeSession } from '../runtime/RuntimeSessions';
import type { Draft } from '../drafts/contract';
import type { AccountWindows, GoalItem, Limit, Used } from '../usage/contract';
import type { DashboardAgent } from './contract';
import type { Ready } from './Dashboard';

const MONDAY = new Date(2026, 9, 12, 0, 0).getTime();
const SATURDAY = new Date(2026, 9, 17, 10, 12).getTime();
const NOW = new Date(2026, 9, 18, 20, 40).getTime();
const ME = 'person-cherie';

const week = (figure: number | null): Used => (figure === null
  ? { unit: 'dollars', period: 'week', since_ms: MONDAY, figure: null, unavailable: 'no cost was reported this week' }
  : { unit: 'dollars', period: 'week', since_ms: MONDAY, figure, unavailable: null });

const plan: AccountWindows = { account: 'cherie@claude', at_ms: NOW, windows: [{ duration_minutes: 300, used_percent: 22, resets_at_ms: NOW + 3_600_000 }] };

const running: RuntimeSession = {
  session: 'session-cherie-helper-1', agent: 'agent-helper', machine: 'machine-cherie-air', machine_name: 'Cherie’s MacBook Air', runtime: 'claude',
  shown: 'running', last_reported: 'running', first_report_at: SATURDAY, last_report_at: NOW, what: 'claude', stopped: null, reported_by: 'machine-cherie-air',
};

const agent = (id: string, name: string, more: { sessions?: RuntimeSession[]; figure: number | null; accounts?: AccountWindows[]; limits?: Limit[]; used?: Used[]; goals?: GoalItem[] }): DashboardAgent => ({
  agent: { id, display_name: name, state: 'active' }, teams: [], sessions: more.sessions ?? [],
  usage: { agent: id, used: [], receipts: [], last_reported_ms: more.figure === null ? null : NOW, figures: [week(more.figure)], accounts: more.accounts ?? [] },
  budget: { holder: { kind: 'agent', id }, limits: more.limits ?? [], warn_at: null, zone: 'Australia/Sydney', version: more.limits ? 1 : 0,
    by: more.limits ? ME : null, at: more.limits ? SATURDAY : null, used: more.used ?? [], unavailable: [], within: [], unconfirmed: [] },
  goals: { goals: more.goals ?? [] },
} as DashboardAgent);

const draft: Draft = {
  id: 'draft-cherie-1', agent: { id: 'agent-helper', display_name: 'Helper' }, responsible: { id: ME, display_name: 'Cherie', state: 'active' },
  target: { kind: 'project', id: 'family-photos', action: 'write' }, method: 'POST', path: '/projects/family-photos/albums', body: '{}',
  note: 'Make an album of Saturday’s photos', created_at: NOW - 1_800_000, creation_hash: 'fixture', state: 'waiting',
} as Draft;

/** Cherie's page, and which of her agents have a program chosen, so a stopped row offers Start or Set up without asking. */
export function cherie(): { data: Ready; setUp: ReadonlyMap<string, boolean> } {
  const agents = [
    agent('agent-helper', 'Helper', { sessions: [running], figure: 4.2, accounts: [plan],
      limits: [{ unit: 'dollars', amount: 5, period: 'week', act: 'tell' }], used: [week(4.2)] }),
    agent('agent-researcher', 'Researcher', { figure: 0.75,
      goals: [{ goal: { id: 'goal-receipts', kind: 'goal', words: 'Sort the receipts folder by Friday', deadline: NOW - 86_400_000 * 2, active: true, evidence: 'document' }, standing: 'missed' } as GoalItem] }),
    agent('agent-sketcher', 'Sketcher', { figure: null }),
  ];
  const me = { person: { id: ME, display_name: 'Cherie', state: 'active' }, signed_in: { provider: 'lys', subject: 'cherie' }, sign_in_identities: [], service_accounts: [] } as unknown as MeView;
  return {
    data: {
      kind: 'active', me, teams: [],
      // What waits on her as GET /waiting answers it: Helper's draft, carrying the draft so its row decides it.
      waiting: [{ kind: 'agent_draft', id: draft.id, asked_by: { id: 'agent-helper', name: 'Helper' }, about: 'Helper prepared something for you to approve: ' + draft.note,
        asked_at: new Date(draft.created_at).toISOString(), can_decide: true, draft }],
      answer: { agents, waiting: { requests: 0, drafts: 1, reviews: 0, product_drafts: 0 }, alarms: { open_count: 0, newest: [] } } as unknown as Ready['answer'],
      cord: { pulled: null, last: null, released: null, may_pull: true },
      budgets: { holder: { kind: 'person', id: ME }, limits: [{ unit: 'dollars', amount: 20, period: 'week', act: 'tell' }], warn_at: null, zone: 'Australia/Sydney', version: 1, by: ME, at: SATURDAY, used: [week(4.95)], unavailable: [], within: [], unconfirmed: [] },
      standing: { machines: 1, app: null, asking: null, apps: false, google: false, read: true, administrator: true, declined: false },
    },
    setUp: new Map([['agent-helper', true], ['agent-researcher', true], ['agent-sketcher', false]]),
  };
}
