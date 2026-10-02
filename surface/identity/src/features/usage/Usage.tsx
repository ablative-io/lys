import { readTogether } from '../../reads';
/** Plain controls for an agent's budgets and goals, on its own file; graphs and analytics stay with the monitoring app. */
import { useState } from 'react';
import { request, useLoad } from '../../api';
import { Gate } from '../signin/Gate';
import { UsageBudgets } from './UsageBudgets';
import { UsageGoals } from './UsageGoals';
import { tracking } from './contract';
import type { BudgetsView, GoalsView, UsageView } from './contract';
import './usage.css';

export function AgentUsage({ agent }: { agent: string }) {
  const [revision, setRevision] = useState(0);
  const [notice, setNotice] = useState('');
  const path = encodeURIComponent(agent);
  const load = useLoad(() => readTogether({
    budgets: request<BudgetsView>('/budgets/agent/' + path),
    usage: request<UsageView>('/agents/' + path + '/usage'),
    goals: request<GoalsView>('/agents/' + path + '/goals'),
  }), 'usage:' + agent + ':' + revision);
  const changed = (words: string) => { setNotice(words); setRevision((value) => value + 1); };
  return <>{notice ? <p role="status" className="usage-notice">{notice}</p> : null}<Gate load={load} title="Usage" ok={({ budgets, usage, goals }) => {
    const tracked = tracking(usage);
    return <>
      <p className="usage-tracking" data-complete={tracked.complete} role="status">{tracked.words}</p>
      <UsageBudgets agent={agent} budgets={budgets} receipts={usage.receipts} changed={changed} />
      <UsageGoals agent={agent} goals={goals.goals} changed={changed} />
    </>;
  }} /></>;
}
