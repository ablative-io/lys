/** The tree of names down the left: each team under its lead, each agent under its team, nothing of yours repeated. A team's name folds its members away and back; a name opens its terminal; the plus beside a running one opens it beside the terminals already open. */
import { useState } from 'react';
import type { CSSProperties } from 'react';
import { useLocation } from 'react-router';
import { pref, setPref } from '../../shell/prefs';
import type { Entry } from '../people/directory';
import type { RuntimeSession } from '../runtime/RuntimeSessions';
import type { Branch } from './tree';
import { addressOf, liveOf, openAgents } from './world';
import type { World } from './world';
import './team.css';

const depth = (level: number) => ({ '--depth': level } as CSSProperties);
const FOLDED = 'team-folded';
const readFolded = () => new Set(pref(FOLDED, '').split(',').filter(Boolean));

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

function Branches({ branches, level, open, sessions, folded, fold }: { branches: Branch[]; level: number; open: string[]; sessions: RuntimeSession[]; folded: Set<string>; fold: (team: string) => void }) {
  return <>{branches.map((branch, index) => {
    const away = branch.team ? folded.has(branch.team.id) : false;
    const running = branch.members.filter((member) => liveOf(sessions, member.entry.id)).length;
    return <div key={branch.team?.id ?? 'loose-' + index}>
      {branch.team ? <button type="button" className="tree-team" style={depth(level)} aria-expanded={!away} onClick={() => fold(branch.team!.id)}>
        <span className="tree-fold" aria-hidden="true" />{branch.team.name}
        {away ? <span className="tree-count">{running ? running + ' of ' : ''}{branch.members.length}</span> : null}
      </button> : null}
      {away ? null : branch.members.map((member) => <div key={member.entry.id}>
        <Name entry={member.entry} level={level} open={open} session={liveOf(sessions, member.entry.id)} />
        <Branches branches={member.branches} level={level + 1} open={open} sessions={sessions} folded={folded} fold={fold} />
      </div>)}
    </div>;
  })}</>;
}

export function TeamTree({ world }: { world: World | null }) {
  const { pathname, search } = useLocation();
  const open = openAgents(pathname, search);
  const [folded, setFolded] = useState(readFolded);
  const fold = (team: string) => {
    const next = new Set(folded);
    if (next.has(team)) next.delete(team); else next.add(team);
    setPref(FOLDED, [...next].join(','));
    setFolded(next);
  };
  return <nav className="team-tree" aria-label="Teams and agents">
    {world ? <Branches branches={world.tree.branches} level={0} open={open} sessions={world.sessions} folded={folded} fold={fold} /> : null}
  </nav>;
}
