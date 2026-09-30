/** The team screen: a tree of names down the left, the chosen agents' terminals filling the rest, split like a multiplexer. Two ways to reach settings are offered side by side: a drawer over the terminal, or a footer of tabs. */
import { useState } from 'react';
import type { ReactNode } from 'react';
import { useParams } from 'react-router';
import { api, request, useLoad } from '../../api';
import { useShell } from '../../shell/ShellContext';
import { entries } from '../people/directory';
import type { Entry } from '../people/directory';
import type { Team as TeamRecord } from '../teams/contract';
import type { RuntimeSession } from '../runtime/RuntimeSessions';
import { Terminal } from '../runtime/Terminal';
import { Provisioning } from '../provisioning/Provisioning';
import { AgentUsage } from '../usage/Usage';
import { Start } from './Start';
import { agentsOf, buildTree } from './tree';
import type { Branch, Node } from './tree';
import '../runtime/terminal.css';
import '../usage/usage.css';
import './team.css';

const TABS = [['terminal', 'Terminal'], ['settings', 'Settings'], ['budgets', 'Budgets and goals']] as const;
type Tab = (typeof TABS)[number][0];

interface World { tree: Node; sessions: RuntimeSession[] }

function useWorld(revision: number) {
  return useLoad(async (): Promise<World> => {
    const [teams, people, me, live] = await Promise.all([
      request<{ teams: TeamRecord[] }>('/teams'), api.people(), api.me(), request<{ sessions: RuntimeSession[] }>('/runtime/live'),
    ]);
    const all = entries(people);
    const root = all.find((entry) => entry.id === me.person.id);
    if (!root) throw new Error('The directory did not list you.');
    return { tree: buildTree(root, teams.teams.filter((team) => team.state === 'active'), all), sessions: live.sessions };
  }, 'team-world:' + revision);
}

/** One name in the tree; the chosen one is marked the way the rail marks the current screen. */
function Name({ entry, depth, base, chosen, running, add, extra }: { entry: Entry; depth: number; base: string; chosen: boolean; running: boolean; add: (id: string) => void; extra: ReactNode }) {
  return <div className="tree-line" style={{ '--depth': depth } as React.CSSProperties}>
    <a className="tree-name" aria-current={chosen ? 'page' : undefined} href={base + encodeURIComponent(entry.id)}>
      <span className={'dot ' + (running ? 's-active' : 's-retired')} aria-label={running ? 'running' : 'not running'} />{entry.display_name}
    </a>
    {running && !chosen ? <button type="button" className="tree-add" title={'Add ' + entry.display_name + ' beside the open terminal'} onClick={() => add(entry.id)}>+</button> : null}
    {chosen ? extra : null}
  </div>;
}

function Branches({ branches, depth, live, base, open, add, extra }: { branches: Branch[]; depth: number; live: (id: string) => RuntimeSession | undefined; base: string; open: string | undefined; add: (id: string) => void; extra: ReactNode }) {
  return <>{branches.map((branch, index) => <div key={branch.team?.id ?? 'loose-' + index}>
    {branch.team ? <div className="tree-team" style={{ '--depth': depth } as React.CSSProperties}>{branch.team.name}</div> : null}
    {branch.members.map((member) => <div key={member.entry.id}>
      <Name entry={member.entry} depth={depth} base={base} chosen={open === member.entry.id} running={!!live(member.entry.id)} add={add} extra={extra} />
      <Branches branches={member.branches} depth={depth + 1} live={live} base={base} open={open} add={add} extra={extra} />
    </div>)}
  </div>)}</>;
}

function Pane({ entry, session, single, started }: { entry: Entry; session: RuntimeSession | undefined; single: boolean; started: () => void }) {
  return session ? <Terminal key={session.session} session={session.session} agent={entry.id} />
    : <div className="team-stopped">{single ? null : <h2>{entry.display_name}</h2>}<Start key={entry.id} agent={entry.id} started={started} /></div>;
}

function Settings({ id }: { id: string }) {
  return <div className="team-settings"><Provisioning id={id} />
    <section className="card"><h3>Runtime</h3>
      <div className="set-row"><div>Restart<div className="d">Ends the session and starts it again from its profile.</div></div><button type="button" className="btn" disabled title="Restart lands tonight.">Restart</button></div>
      <div className="set-row"><div>Ask for an MCP server<div className="d">A request to the lead; an approval becomes a new profile version.</div></div><button type="button" className="btn" disabled title="Requests land tonight.">Ask</button></div>
    </section>
    <AgentUsage agent={id} /></div>;
}

function Screen({ variant }: { variant: 'drawer' | 'footer' }) {
  const { agent: open, tab: asked } = useParams();
  const tab: Tab = TABS.some(([id]) => id === asked) ? asked as Tab : 'terminal';
  const shell = useShell();
  const [revision, setRevision] = useState(0);
  const [extra, setExtra] = useState<string[]>([]);
  const base = variant === 'drawer' ? '#/team/' : '#/team-tabs/';
  const load = useWorld(revision);
  if (load.status === 'loading') return <div className="page" />;
  if (load.status === 'refused') {
    const { status, refusal } = load.refused;
    return <p className="page why-not">Could not read your teams and agents: {status === 401 ? 'you are not signed in. Sign in to Lys, then reload this page.' : status ? `the service answered ${status}: ${refusal.reason}` : refusal.reason}</p>;
  }
  const { tree, sessions } = load.data;
  const agents = agentsOf(tree);
  const live = (id: string) => sessions.find((entry) => entry.agent === id && entry.shown !== 'stopped');
  const chosen = open ? agents.find((entry) => entry.id === open) : undefined;
  const panes = chosen ? [chosen, ...extra.filter((id) => id !== chosen.id).flatMap((id) => agents.filter((entry) => entry.id === id))] : [];
  const add = (id: string) => setExtra((ids) => ids.includes(id) || ids.length >= 3 ? ids : [...ids, id]);
  const started = () => setRevision((value) => value + 1);
  const gear = variant === 'drawer' && chosen
    ? <button type="button" className="tree-gear" title={'Settings of ' + chosen.display_name} onClick={() => shell.openDrawer(<Settings id={chosen.id} />)} aria-label="Settings">⚙</button>
    : null;
  return <div className="page team-page">
    <nav className="team-tree" aria-label="Teams and agents">
      <div className="tree-team tree-root">{tree.entry.display_name}</div>
      <Branches branches={tree.branches} depth={1} live={live} base={base} open={open} add={add} extra={gear} />
    </nav>
    <div className="team-stage">
      {!chosen ? <p className="dim team-empty">Choose an agent.</p> : variant === 'footer' && tab === 'settings' ? <div className="team-scroll"><Provisioning id={chosen.id} /></div>
        : variant === 'footer' && tab === 'budgets' ? <div className="team-scroll"><AgentUsage agent={chosen.id} /></div>
        : <div className="team-panes" data-n={panes.length}>{panes.map((entry) => <div className="team-pane" key={entry.id}>
          {panes.length > 1 ? <button type="button" className="pane-close" title={'Close ' + entry.display_name + "'s pane; it keeps running"} onClick={() => entry.id === chosen.id ? undefined : setExtra((ids) => ids.filter((id) => id !== entry.id))} disabled={entry.id === chosen.id}>×</button> : null}
          <Pane entry={entry} session={live(entry.id)} single={panes.length === 1} started={started} />
        </div>)}</div>}
      {variant === 'footer' && chosen ? <nav className="tabs team-foot" aria-label="Agent views">{TABS.map(([id, words]) => <a key={id} className={tab === id ? 'on' : undefined} href={base + encodeURIComponent(chosen.id) + '/' + id}>{words}</a>)}</nav> : null}
    </div>
  </div>;
}

export const Team = () => <Screen variant="drawer" />;
export const TeamTabs = () => <Screen variant="footer" />;
