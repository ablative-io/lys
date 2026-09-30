/** The terminal is the screen: the open agents' terminals fill it edge to edge, split like a multiplexer, each with its own foot line. The tree of names is down the left and folds away; an agent's settings open over its own terminal and close from a button or Escape. */
import { useEffect, useState } from 'react';
import type { KeyboardEvent } from 'react';
import { useLocation, useNavigate } from 'react-router';
import { Refused, request } from '../../api';
import { pref, setPref } from '../../shell/prefs';
import type { Entry } from '../people/directory';
import type { RuntimeSession } from '../runtime/RuntimeSessions';
import { Terminal } from '../runtime/Terminal';
import { Provisioning } from '../provisioning/Provisioning';
import { AgentUsage } from '../usage/Usage';
import { Start } from './Start';
import { agentsOf } from './tree';
import { TeamTree } from './TeamTree';
import { addressOf, liveOf, openAgents, useWorld } from './world';
import '../runtime/terminal.css';
import '../usage/usage.css';
import './team.css';

const TREE = 'team-tree';

function Runtime({ session, changed }: { session: RuntimeSession | undefined; changed: () => void }) {
  const [said, setSaid] = useState('');
  const [busy, setBusy] = useState(false);
  const stop = async () => {
    if (!session) return;
    setBusy(true);
    try {
      await request('/runtime/sessions/' + encodeURIComponent(session.session) + '/end', {});
      setSaid('Stopped.');
      changed();
    } catch (error) {
      setSaid(error instanceof Refused ? error.refusal.refusal + ': ' + error.refusal.reason : String(error));
    }
    setBusy(false);
  };
  return <section className="card"><h3>Runtime</h3>
    <div className="set-row">
      <div>{session ? 'Running on ' + (session.machine_name ?? session.machine) : 'Not running'}{session ? <div className="d mono">{session.session}</div> : null}</div>
      {session ? <button type="button" className="btn danger" disabled={busy} onClick={() => void stop()}>Stop</button> : null}
    </div>
    <div className="set-row"><div>Restart<div className="d">Ends the session and starts it again from its profile.</div></div><button type="button" className="btn" disabled title="Restart lands tonight.">Restart</button></div>
    <div className="set-row"><div>Ask for an MCP server<div className="d">A request to the lead; an approval becomes a new profile version.</div></div><button type="button" className="btn" disabled title="Requests land tonight.">Ask</button></div>
    {said ? <p role="status" className="note">{said}</p> : null}
  </section>;
}

function Settings({ entry, session, changed, done }: { entry: Entry; session: RuntimeSession | undefined; changed: () => void; done: () => void }) {
  const onKey = (event: KeyboardEvent<HTMLDivElement>) => {
    if (event.key === 'Escape') { event.stopPropagation(); done(); }
  };
  return <div className="team-settings" role="dialog" aria-label={entry.display_name + ' settings'} tabIndex={-1} onKeyDown={onKey} ref={(el) => el?.focus()}>
    <div className="team-settings-head"><h2>{entry.display_name}</h2><button type="button" className="btn" onClick={done}>Back to the terminal</button></div>
    <Runtime session={session} changed={changed} />
    <Provisioning id={entry.id} />
    <AgentUsage agent={entry.id} />
  </div>;
}

function Pane({ entry, session, alone, started }: { entry: Entry; session: RuntimeSession | undefined; alone: boolean; started: () => void }) {
  return session
    ? <Terminal key={session.session} session={session.session} agent={entry.id} bare />
    : <div className="team-stopped">{alone ? null : <p className="team-start-line">{entry.display_name}</p>}<Start key={entry.id} agent={entry.id} name={entry.display_name} started={started} /></div>;
}

export function Team() {
  const { pathname, search } = useLocation();
  const open = openAgents(pathname, search);
  const [revision, setRevision] = useState(0);
  const [treeShown, setTree] = useState(() => pref(TREE, 'shown'));
  const [settingsFor, setSettingsFor] = useState<string | null>(null);
  const toggleTree = () => {
    const next = treeShown === 'shown' ? 'hidden' : 'shown';
    setPref(TREE, next);
    setTree(next);
  };
  const load = useWorld(revision);
  const navigate = useNavigate();
  const first = load.status === 'ok' ? agentsOf(load.data.tree).find((entry) => liveOf(load.data.sessions, entry.id)) ?? agentsOf(load.data.tree)[0] : undefined;
  useEffect(() => {
    if (!open.length && first) navigate('/team/' + encodeURIComponent(first.id), { replace: true });
  }, [open.length, first?.id]);
  if (load.status === 'loading') return <div className="team-screen" data-tree={treeShown}><TeamTree world={null} /></div>;
  if (load.status === 'refused') {
    const { status, refusal } = load.refused;
    return <p className="team-screen why-not team-refused">Could not read your teams and agents: {status === 401 ? 'you are not signed in. Sign in to Lys, then reload this page.' : status ? `the service answered ${status}: ${refusal.reason}` : refusal.reason}</p>;
  }
  const { tree, sessions } = load.data;
  const agents = agentsOf(tree);
  const panes = open.flatMap((id) => agents.filter((entry) => entry.id === id));
  const changed = () => setRevision((value) => value + 1);
  const shown = treeShown === 'shown';
  return <div className="team-screen" data-tree={treeShown}><TeamTree world={load.data} /><div className="team-stage">
    {panes.length
      ? <div className="team-panes" data-n={panes.length}>{panes.map((entry, index) => {
        const session = liveOf(sessions, entry.id);
        const inSettings = settingsFor === entry.id;
        return <div className="team-pane" key={entry.id}>
          <div className="team-pane-body">
            <Pane entry={entry} session={session} alone={panes.length === 1} started={changed} />
            {inSettings ? <Settings entry={entry} session={session} changed={changed} done={() => setSettingsFor(null)} /> : null}
          </div>
          <div className="team-foot">
            {index === 0 ? <button type="button" className="icon-btn team-foot-tree" aria-pressed={shown} title={shown ? 'Hide the list of agents' : 'Show the list of agents'} onClick={toggleTree}>{shown ? '‹' : '›'}</button> : null}
            <span className="team-foot-name">{entry.display_name}</span>
            {session ? <span className="team-foot-machine">on {session.machine_name ?? session.machine}</span> : null}
            <button type="button" className="team-foot-act" aria-pressed={inSettings} onClick={() => setSettingsFor(inSettings ? null : entry.id)}>{inSettings ? 'Back to the terminal' : 'Settings'}</button>
            {panes.length > 1 ? <a className="team-foot-act" href={addressOf(open.filter((id) => id !== entry.id))} title="Close this pane; the agent keeps running">Close</a> : null}
          </div>
        </div>;
      })}</div>
      : <p className="team-empty dim">{agents.length ? 'Choose an agent from the list.' : 'No agent answers to you yet.'}</p>}
  </div></div>;
}
