/** The budget, usage and goal shapes the identity service answers for one agent. */
export type Measure = 'context_percent' | 'tokens' | 'running_ms' | 'dollars' | 'plan_percent';
export type BudgetAct = 'compact' | 'notice' | 'stop' | 'tell';
export type Length = 'five_hour' | 'day' | 'week' | 'month';
export type Period = { length: Length; zone: string };
export type Holder = { kind: 'agent' | 'team' | 'person'; id: string };
export type Budget = { holder: Holder; measure: Measure; limit: number; period: Period | null; act: BudgetAct; version: number; by: string; at: number };
export type Limit = { unit: Measure; amount: number; period: Length | null; act: BudgetAct; zone?: string };
export type Used = { unit: Measure; period: Length | null; since_ms: number | null } & ({ figure: number; unavailable: null } | { figure: null; unavailable: string });
export type BudgetBody = { limits: Limit[]; warn_at: number | null; version: number };
export type Within = { team: string; name: string; limits: Limit[]; used: Used[] };
export type Unconfirmed = { requested: Budget; effective: Budget; reason: string };
export type BudgetsView = { holder: Holder; limits: Limit[]; warn_at: number | null; zone: string; version: number; by: string | null; at: number | null; used: Used[]; unavailable: { unit: Measure; reason: string }[]; within: Within[]; effective_limits?: Limit[]; unconfirmed: Unconfirmed[] };
export type Stands = 'accepted' | 'delivered' | 'confirmed' | 'uncertain' | 'refused' | 'told';
export type Crossing = { operation: string; holder: Holder; measure: Measure; version: number; limit: number; figure: number; limit_index: number; warning: boolean; account?: string; act: BudgetAct; agent: string; at_ms: number };
export type Receipt = { crossing: Crossing; acted: { stands: Stands; words: string; at_ms: number } | null };
export type UsageView = { agent: string; used: Used[]; receipts: Receipt[]; last_reported_ms: number | null };
export type GoalKind = 'goal' | 'expectation' | 'deliverable';
export type Standing = 'open' | 'met' | 'missed' | 'dropped';
export type GoalItem = { goal: { id: string; kind: GoalKind; words: string; deadline: number | null; active: boolean; evidence: 'commit' | 'document' | 'check' | null }; standing: Standing };
export type GoalsView = { goals: GoalItem[] };

export const MEASURES: Record<Measure, string> = {
  dollars: 'Reported dollar spend',
  plan_percent: 'Percent of the shared account plan',
  context_percent: 'Context, in percent of the window',
  tokens: 'Tokens in a period',
  running_ms: 'Running time in a period, in minutes',
};

export const ACTS: Record<BudgetAct, string> = {
  compact: 'Compact the session',
  notice: 'Type a notice into the session',
  stop: 'End the session',
  tell: 'Tell the responsible person',
};

/** A limit as the screen shows and takes it: running time in minutes, everything else as kept. */
export const shown = (measure: Measure, limit: number): number => (measure === 'running_ms' ? limit / 60000 : limit);
export const kept = (measure: Measure, given: number): number => (measure === 'running_ms' ? Math.round(given * 60000) : given);

/** Where a budget stands, in plain words, from the latest receipt of its version's crossing. */
export function standing(budget: Budget, receipts: Receipt[]): { reached: boolean; words: string } {
  const crossed = receipts.filter((r) => r.crossing.measure === budget.measure && r.crossing.version === budget.version
    && r.crossing.holder.kind === budget.holder.kind && r.crossing.holder.id === budget.holder.id);
  const latest = crossed[crossed.length - 1];
  if (!latest) return { reached: false, words: 'Held: not reached.' };
  const act = ACTS[latest.crossing.act].toLowerCase();
  const figure = 'Reached at ' + shown(budget.measure, latest.crossing.figure) + '.';
  if (!latest.acted) return { reached: true, words: figure + ' Nothing is known yet of its act (' + act + ').' };
  switch (latest.acted.stands) {
    case 'confirmed': return { reached: true, words: figure + ' Its act was confirmed: ' + act + '.' };
    case 'uncertain': return { reached: true, words: figure + ' Whether its act (' + act + ') reached the session cannot be known. It is not shown done and is not asked again.' };
    case 'accepted':
    case 'delivered': return { reached: true, words: figure + ' Its act (' + act + ') is pending and not confirmed.' };
    case 'refused': return { reached: true, words: figure + ' Its act (' + act + ') was refused: ' + latest.acted.words };
    case 'told': return { reached: true, words: figure + ' The responsible person was told.' };
  }
}

/** Whether the agent's runner reports usage, in plain words. */
export function tracking(usage: UsageView): { complete: boolean; words: string } {
  if (usage.last_reported_ms === null) {
    return { complete: false, words: 'Tracking is incomplete: no usage has been reported for this agent, so its budgets cannot be reached.' };
  }
  return { complete: true, words: 'Usage last reported ' + new Date(usage.last_reported_ms).toLocaleString() + '.' };
}
