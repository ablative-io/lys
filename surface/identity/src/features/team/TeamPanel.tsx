/** The tree of names down the left: you at the root, each team under its lead, each agent under its team. A name opens its terminal; the plus beside a running one opens it beside the terminals already open. */
import type { CSSProperties } from 'react';
import { useLocation } from 'react-router';
import type { Entry } from '../people/directory';
import type { RuntimeSession } from '../runtime/RuntimeSessions';
import type { Branch } from './tree';
import { addressOf, liveOf, openAgents, useWorld } from './world';
import './team.css';

const depth = (level: number) => ({ '--depth': level } as CSSProperties);

function Name({ entry, level, open, session }: { entry: Entry; level: number; open: string[]; session: RuntimeSession | undefined }) {
  const chosen = open[0] === entry.id;
  const shown = open.includes(entry.id);
  return <div className="tree-line" style={depth(level)}>
    <a className="tree-name" aria-current={chosen ? 'page' : shown ? 'true' : undefined} href={addressOf([entry.id])}>
      <span className={'dot ' + (session ? 's-active' : 's-retired')} aria-label={session ? 'running' : 'not running'} />{entry.display_name}
    </a>
    {session && open.length > 0 && !shown && open.length < 4 ? <a className="tree-add" title="Open beside the terminals already open" href={addressOf([...open, entry.id])}>+</a> : null}
  </div>;
}

function Branches({ branches, level, open, sessions }: { branches: Branch[]; level: number; open: string[]; sessions: RuntimeSession[] }) {
  return <>{branches.map((branch, index) => <div key={branch.team?.id ?? 'loose-' + index}>
    {branch.team ? <div className="tree-team" style={depth(level)}>{branch.team.name}</div> : null}
    {branch.members.map((member) => <div key={member.entry.id}>
      <Name entry={member.entry} level={level} open={open} session={liveOf(sessions, member.entry.id)} />
      <Branches branches={member.branches} level={level + 1} open={open} sessions={sessions} />
    </div>)}
  </div>)}</>;
}

export function TeamTree() {
  const { pathname, search } = useLocation();
  const open = openAgents(pathname, search);
  const load = useWorld(0);
  return <nav className="team-tree" aria-label="Teams and agents">
    {load.status === 'loading' ? null : load.status === 'refused'
      ? <p className="why-not">{load.refused.status === 401 ? 'Sign in to Lys to see your agents.' : load.refused.refusal.reason}</p>
      : <>
        <div className="tree-team tree-root">{load.data.tree.entry.display_name}</div>
        <Branches branches={load.data.tree.branches} level={1} open={open} sessions={load.data.sessions} />
      </>}
  </nav>;
}
