/** Plain controls for an agent's budgets and goals; graphs and analytics stay with the monitoring app. */
import { useState } from 'react';
import { useParams } from 'react-router';
import { api, request, useLoad } from '../../api';
import { Gate } from '../signin/Gate';
import { entries } from '../people/directory';
import { UsageBudgets } from './UsageBudgets';
import { UsageGoals } from './UsageGoals';
import { tracking } from './contract';
import type { BudgetsView, GoalsView, UsageView } from './contract';
import './usage.css';

export function Usage() {
  const { agent } = useParams();
  const people = useLoad(api.people, 'usage-people');
  return <section className="usage"><div className="head"><div><h2>Usage</h2><p>Each agent's budgets and goals, and where each stands. Graphs and analytics are in the monitoring app.</p></div></div>
    <Gate load={people} title="Usage" ok={(view) => {
      const agents = entries(view).filter((entry) => entry.kind === 'agent');
      return <>
        <nav className="usage-agents" aria-label="Agents">{agents.length ? agents.map((entry) => <a key={entry.id} href={'#/usage/' + entry.id} aria-current={entry.id === agent ? 'page' : undefined}>{entry.display_name}</a>) : <p>No agents are visible to you.</p>}</nav>
        {agent ? <AgentUsage key={agent} agent={agent} /> : <p>Choose an agent to see its budgets and goals.</p>}
      </>;
    }} />
  </section>;
}

export function AgentUsage({ agent }: { agent: string }) {
  const [revision, setRevision] = useState(0);
  const [notice, setNotice] = useState('');
  const path = encodeURIComponent(agent);
  const load = useLoad(async () => ({
    budgets: await request<BudgetsView>('/budgets/agent/' + path),
    usage: await request<UsageView>('/agents/' + path + '/usage'),
    goals: await request<GoalsView>('/agents/' + path + '/goals'),
  }), 'usage:' + agent + ':' + revision);
  const changed = (words: string) => { setNotice(words); setRevision((value) => value + 1); };
  return <>{notice ? <p role="status" className="usage-notice">{notice}</p> : null}<Gate load={load} title="Usage" ok={({ budgets, usage, goals }) => {
    const tracked = tracking(usage);
    return <>
      <p className="usage-tracking" data-complete={tracked.complete} role="status">{tracked.words}</p>
      <UsageBudgets agent={agent} budgets={budgets.budgets} receipts={usage.receipts} changed={changed} />
      <UsageGoals agent={agent} goals={goals.goals} changed={changed} />
    </>;
  }} /></>;
}
