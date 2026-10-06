import { readTogether } from '../../reads';
/** Plain controls for an agent's budgets and goals on its own file, and a team's on the Teams page; graphs and analytics stay with the monitoring app. */
import { useState } from 'react';
import { request, useLoad } from '../../api';
import { Gate } from '../signin/Gate';
import { UsageBudgets } from './UsageBudgets';
import { UsageGoals } from './UsageGoals';
import { tracking } from './contract';
import type { ProvisioningAnswer } from '../provisioning/Provisioning';
import type { BudgetsView, GoalsView, UsageView } from './contract';
import './usage.css';

/** `name` is the agent's name, said in place of its identifier wherever the service's words carry it. */
export function AgentUsage({ agent, name }: { agent: string; name?: string }) {
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
      <ControlSetup agent={agent} />
      <p className="usage-tracking" data-complete={tracked.complete} role="status">{tracked.words}</p>
      <UsageBudgets budgets={budgets} receipts={usage.receipts} changed={changed} name={name} />
      <UsageGoals agent={agent} goals={goals.goals} changed={changed} />
    </>;
  }} /></>;
}

/** A team's limits and goals: the same two tables as an agent's. A team's use is read with its limits. */
export function TeamUsage({ team, name }: { team: string; name?: string }) {
  const [revision, setRevision] = useState(0);
  const [notice, setNotice] = useState('');
  const path = encodeURIComponent(team);
  const load = useLoad(() => readTogether({
    budgets: request<BudgetsView>('/budgets/team/' + path),
    goals: request<GoalsView>('/teams/' + path + '/goals'),
  }), 'team-usage:' + team + ':' + revision);
  const changed = (words: string) => { setNotice(words); setRevision((value) => value + 1); };
  return <div className="usage">{notice ? <p role="status" className="usage-notice">{notice}</p> : null}<Gate load={load} title="Team limits and goals" ok={({ budgets, goals }) => <>
    <UsageBudgets budgets={budgets} receipts={[]} changed={changed} name={name} />
    <UsageGoals agent={team} kind="team" goals={goals.goals} changed={changed} />
  </>} /></div>;
}

/** The saved setup chooses the next start; a running session is never moved by a read. */
function ControlSetup({ agent }: { agent: string }) {
  const load = useLoad(async () => {
    const answer = await request<ProvisioningAnswer>('/agents/' + encodeURIComponent(agent) + '/provisioning');
    const required = answer.profile?.session?.requires_controls;
    if (answer.agent !== agent || (required !== undefined && typeof required !== 'boolean')) throw new Error('ControlSetupUnreadable: the saved setup did not name this agent and its control requirement.');
    return required === true;
  }, 'control-setup:' + agent);
  return <Gate load={load} title="Control setup" renderError={(failure) => <p role="status">Control setup unavailable: {failure.refusal.refusal}: {failure.message}</p>} ok={(required) => <p role="status">{required ? 'Managed controls are required by the saved setup. No managed adapter is qualified; starts are refused (control_adapter_unqualified).' : 'Notices use the terminal. No managed adapter is qualified.'}</p>} />;
}
