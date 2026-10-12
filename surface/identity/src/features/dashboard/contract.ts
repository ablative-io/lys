/** The one answer the Dashboard reads, `GET /dashboard`: the person's agents with their teams, live sessions, usage, budget and goals, and how much waits for the person. Each part answers as its own route does, or by that route's refusal. */
import type { AgentSummary } from '../../generated';
import { readEveryPage } from '../../reads';
import type { Paged } from '../../reads';
import type { RuntimeSession } from '../runtime/RuntimeSessions';
import type { BudgetsView, GoalsView, UsageView } from '../usage/contract';
import type { Alarm, AlarmSummary } from './Alarms';

/** A part the service could not read, by the service's own refusal name and reason. */
export interface PartRefused { refusal: string; reason: string }

export interface DashboardAgent {
  agent: AgentSummary;
  /** The ids of the teams in use the agent is an admitted member of; their names are read from `/teams`. */
  teams: string[] | PartRefused;
  /** Its sessions not confirmed stopped, as last reported. */
  sessions: RuntimeSession[] | PartRefused;
  /** The answer of `GET /agents/{id}/usage`. */
  usage: UsageView | PartRefused;
  /** The answer of `GET /budgets/agent/{id}`: the agent's limits, each with what is used against it. */
  budget: BudgetsView | PartRefused;
  /** The answer of `GET /agents/{id}/goals`. */
  goals: GoalsView | PartRefused;
}

/** How many wait on the person, each counted from its own route, or that route's refusal. */
export interface Waiting { requests: number | PartRefused; drafts: number | PartRefused; reviews: number | PartRefused }

export interface DashboardAnswer extends Paged { agents: DashboardAgent[]; waiting: Waiting; alarms: AlarmSummary | PartRefused }

export const refusedPart = (part: unknown): part is PartRefused =>
  !!part && typeof part === 'object' && !Array.isArray(part) && typeof (part as PartRefused).refusal === 'string';

const counted = (value: unknown): boolean => refusedPart(value) || (Number.isSafeInteger(value) && (value as number) >= 0);

/** An alarm is displayed only when its identity and lifecycle are complete. */
export function validAlarm(value: unknown): value is Alarm {
  if (!value || typeof value !== 'object') return false;
  const alarm = value as Partial<Alarm>;
  const instant = (at: unknown): at is number => Number.isSafeInteger(at) && (at as number) >= 0;
  const word = (text: unknown): text is string => typeof text === 'string' && text.trim().length > 0;
  return [alarm.id, alarm.session, alarm.agent, alarm.machine, alarm.detail].every(word)
    && (alarm.kind === 'seat_stalled' || alarm.kind === 'session_ungoverned')
    && instant(alarm.raised_at)
    && ((alarm.acknowledged_by === null && alarm.acknowledged_at === null)
      || (word(alarm.acknowledged_by) && instant(alarm.acknowledged_at) && alarm.acknowledged_at >= alarm.raised_at))
    && (alarm.cleared_at === null || (instant(alarm.cleared_at) && alarm.cleared_at >= alarm.raised_at));
}

/** Read the dashboard, every page of it, and refuse an answer it cannot stand behind, so nothing missing is shown as zero or as nothing. */
export async function readDashboard(): Promise<DashboardAnswer> {
  const answer = await readEveryPage<DashboardAnswer>('/dashboard', (read, page) => {
    if (!page || !Array.isArray(page.agents)) throw new Error('DashboardUnreadable: a later page did not answer a list of agents.');
    return { ...read, agents: [...read.agents, ...page.agents] };
  });
  if (!answer || !Array.isArray(answer.agents)) throw new Error('DashboardUnreadable: the service did not answer a list of agents.');
  const waiting = answer.waiting as Partial<Waiting> | undefined;
  if (!waiting || !counted(waiting.requests) || !counted(waiting.drafts) || !counted(waiting.reviews)) {
    throw new Error('DashboardUnreadable: the service did not say how many requests, drafts and reviews wait for you.');
  }
  for (const row of answer.agents) {
    if (!row || !row.agent || typeof row.agent.id !== 'string' || !row.usage || !row.goals || !row.budget) throw new Error('DashboardUnreadable: an agent row is missing its agent, usage, budget or goals.');
    if (!Array.isArray(row.teams) && !refusedPart(row.teams)) throw new Error('DashboardUnreadable: the teams of ' + row.agent.display_name + ' were not a list.');
    if (!Array.isArray(row.sessions) && !refusedPart(row.sessions)) throw new Error('DashboardUnreadable: the sessions of ' + row.agent.display_name + ' were not a list.');
  }
  if (!refusedPart(answer.alarms) && (!answer.alarms || !Number.isSafeInteger(answer.alarms.open_count)
    || answer.alarms.open_count < 0 || !Array.isArray(answer.alarms.newest)
    || answer.alarms.newest.length > 3 || answer.alarms.open_count < answer.alarms.newest.length
    || !answer.alarms.newest.every((alarm) => validAlarm(alarm) && alarm.cleared_at === null))) {
    answer.alarms = { refusal: 'AlarmsUnreadable', reason: 'The service did not answer the open alarm count and newest alarms.' };
  }
  return answer;
}
