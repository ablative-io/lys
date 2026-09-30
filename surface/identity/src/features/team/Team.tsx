/** The organisation as a tree: each team under its parent, its lead and members, and one agent open beside it. */
import { useState } from 'react';
import { example } from './example';
import type { TreeAgent, TreeTeam } from './example';
import './team.css';

const tokens = (count: number) => (count >= 1_000_000 ? (count / 1_000_000).toFixed(2) + 'M' : Math.round(count / 1000) + 'k');

function Spend({ agent }: { agent: TreeAgent }) {
  const share = Math.min(1, agent.budget.spent / agent.budget.limit);
  const tone = share >= 0.9 ? 'hot' : share >= 0.6 ? 'warm' : 'cool';
  return <span className="tree-spend" title={tokens(agent.budget.spent) + ' of ' + tokens(agent.budget.limit) + ' tokens'}>
    <span className={'tree-bar ' + tone}><span style={{ width: (share * 100).toFixed(1) + '%' }} /></span>
    <span className="tree-num">{tokens(agent.budget.spent)} / {tokens(agent.budget.limit)}</span>
  </span>;
}

function Row({ agent, lead, open, choose }: { agent: TreeAgent; lead?: boolean; open: string; choose: (id: string) => void }) {
  return <button type="button" className={'tree-row' + (open === agent.id ? ' on' : '')} onClick={() => choose(agent.id)} aria-pressed={open === agent.id}>
    <span className={'dot ' + (agent.session === 'live' ? 's-active' : 's-retired')} aria-label={agent.session} />
    <span className="tree-name">{agent.name}</span>
    {lead ? <span className="kind agent">lead</span> : null}
    <span className="tree-goal">{agent.goals[0]}</span>
    <Spend agent={agent} />
  </button>;
}

function Branch({ team, open, choose }: { team: TreeTeam; open: string; choose: (id: string) => void }) {
  return <li className="tree-team">
    <div className="tree-label"><span className="eyebrow">{team.name}</span></div>
    <Row agent={team.lead} lead open={open} choose={choose} />
    {team.members.length || team.teams.length ? <ul className="tree-under">
      {team.members.map((member) => <li key={member.id}><Row agent={member} open={open} choose={choose} /></li>)}
      {team.teams.map((child) => <Branch key={child.id} team={child} open={open} choose={choose} />)}
    </ul> : null}
  </li>;
}

function everyone(teams: TreeTeam[]): TreeAgent[] {
  return teams.flatMap((team) => [team.lead, ...team.members, ...everyone(team.teams)]);
}

function Panel({ agent }: { agent: TreeAgent }) {
  const { profile } = agent;
  const later = 'Shown here first; the action is wired by its brief.';
  return <aside className="tree-panel" aria-label={agent.name}>
    <div className="head"><div><div className="eyebrow">{agent.role}</div><h2>{agent.name}</h2></div>
      <span className={'state ' + (agent.session === 'live' ? 'active' : 'retired')}>{agent.session}</span></div>
    <dl className="tree-facts">
      <dt>Goals</dt><dd>{agent.goals.map((goal) => <div key={goal}>{goal}</div>)}</dd>
      <dt>Budget</dt><dd><Spend agent={agent} /> <span className="dim">tokens, held by the {agent.budget.holder}</span></dd>
      <dt>Profile</dt><dd>{profile.version} <span className="dim">reviewed by {profile.reviewed_by}</span></dd>
      <dt>Harness</dt><dd>{profile.harness} <span className="dim">{profile.program}, {profile.model}</span></dd>
      <dt>MCP servers</dt><dd>{profile.mcp_servers.map((server) => <span className="pill" key={server}>{server}</span>)}</dd>
      <dt>Writes to</dt><dd>{profile.writable ? profile.writable.map((folder) => <code key={folder}>{folder}</code>) : <span className="dim">not confined (lead)</span>}</dd>
    </dl>
    <div className="tree-acts">
      <button type="button" className="btn" disabled title={later}>Ask for an MCP server</button>
      <button type="button" className="btn" disabled title={later}>Approve within my remit</button>
      <button type="button" className="btn" disabled title={later}>Restart on latest version</button>
    </div>
    <div className="tree-term" role="img" aria-label="Terminal">
      <span className="dim">{agent.name}'s terminal opens here. Only the terminal on screen streams; every other one is quiet.</span>
    </div>
  </aside>;
}

export function Team() {
  const agents = everyone(example.teams);
  const [open, setOpen] = useState('pikelet');
  const chosen = agents.find((agent) => agent.id === open) ?? agents[0];
  return <div className="page">
    <div className="head"><div><h1>Team</h1><p className="sub">Everyone under {example.root.name}. Choose an agent to see its settings and terminal.</p></div>
      <span className="kind">example data</span></div>
    <div className="tree-split">
      <ul className="tree" aria-label="Teams">
        <li className="tree-root"><span className="kind">root</span> <span className="tree-name">{example.root.name}</span></li>
        {example.teams.map((team) => <Branch key={team.id} team={team} open={chosen.id} choose={setOpen} />)}
      </ul>
      <Panel agent={chosen} />
    </div>
  </div>;
}
