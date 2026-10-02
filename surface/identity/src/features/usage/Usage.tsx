/** Plain controls for an agent's budgets and goals, on its own file; graphs and analytics stay with the monitoring app. */
import { useState } from 'react';
import { request, useLoad } from '../../api';
import { Gate } from '../signin/Gate';
import { UsageBudgets } from './UsageBudgets';
import { UsageGoals } from './UsageGoals';
import { tracking } from './contract';
import type { UsageView } from './contract';
import { eventWords } from './limits';
import type { GoalList, LimitsView, Used } from './limits';
import './usage.css';

export function AgentUsage({ agent }: { agent: string }) {
  const [revision, setRevision] = useState(0);
  const [notice, setNotice] = useState('');
  const path = encodeURIComponent(agent);
  const load = useLoad(async () => ({
    budget: await request<LimitsView>('/budgets/agent/' + path),
    usage: await request<UsageView & { used: Used[] }>('/agents/' + path + '/usage'),
    goals: await request<GoalList>('/agents/' + path + '/goals'),
  }), 'usage:' + agent + ':' + revision);
  const changed = (words: string) => { setNotice(words); setRevision((value) => value + 1); };
  return <>{notice ? <p role="status" className="usage-notice">{notice}</p> : null}<Gate load={load} title="Budget and goals" ok={({ budget, usage, goals }) => {
    const tracked = tracking(usage);
    const latest = usage.receipts[usage.receipts.length - 1];
    return <>
      {tracked.complete ? null : <p className="usage-tracking" role="status">{tracked.words}</p>}
      <UsageGoals agent={agent} goals={goals.goals} changed={changed} />
      <div>
        <UsageBudgets key={budget.version} agent={agent} budget={budget} used={usage.used} changed={changed} />
        {latest ? <p className="usage-event">{eventWords(latest.crossing.measure, latest.crossing.figure, latest.crossing.at_ms, latest.crossing.act, latest.acted?.stands ?? null)}</p> : null}
      </div>
    </>;
  }} /></>;
}
