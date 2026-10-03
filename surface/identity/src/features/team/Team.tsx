/** The front page. The terminal is the screen: the open agents' terminals fill it edge to edge, split like a multiplexer, each with its own foot line. The tree of names is down the left and folds away. A stopped agent shows Start in its own pane; a running one has Stop on its foot line; an agent's settings open over its own terminal and close from a button or Escape. */
import { useEffect, useRef, useState } from 'react';
import type { KeyboardEvent } from 'react';
import { useLocation, useNavigate } from 'react-router';
import { Refused, operationId, request } from '../../api';
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

/** What a refused act says: the reason as the service gave it, its name small after it. */
function Said({ refused }: { refused: Refused | null }) {
  return refused ? <p role="alert" className="why-not">{refused.message} <small className="refusal-name">{refused.refusal.refusal}</small></p> : null;
}
const asRefused = (error: unknown): Refused =>
  error instanceof Refused ? error : new Refused(0, { refusal: 'Unexpected', reason: String(error) });

/** Stop ends the run on its computer and leaves the agent ready to start again. It is not the emergency stop, which pauses the agent. */
function Stop({ entry, session, changed, done }: { entry: Entry; session: RuntimeSession; changed: () => void; done: () => void }) {
  const [busy, setBusy] = useState(false);
  const [refused, setRefused] = useState<Refused | null>(null);
  const stop = async () => {
    setBusy(true); setRefused(null);
    try {
      await request('/runtime/sessions/' + encodeURIComponent(session.session) + '/end', {});
      done();
      changed();
    } catch (error) { setRefused(asRefused(error)); }
    setBusy(false);
  };
  return <div className="team-stop" role="alertdialog" aria-label={'Stop ' + entry.display_name}>
    <p>Stop {entry.display_name}? What it has not saved is lost. You can start it again afterwards.</p>
    <Said refused={refused} />
    <button type="button" className="btn danger" disabled={busy} onClick={() => { void stop(); }}>Stop {entry.display_name}</button>
    <button type="button" className="btn" onClick={done}>Keep it running</button>
  </div>;
}

/** Restart ends the run and starts it again in the same folder, on the same computer, from its approved settings. */
function Restart({ entry, session, changed }: { entry: Entry; session: RuntimeSession; changed: () => void }) {
  const [busy, setBusy] = useState(false);
  const [refused, setRefused] = useState<Refused | null>(null);
  // One request, kept while this is shown, so a second press after no answer sends the same one.
  const operation = useRef('');
  const restart = async () => {
    setBusy(true); setRefused(null);
    try {
      operation.current ||= operationId();
      await request('/agents/' + encodeURIComponent(entry.id) + '/restart', { session: session.session, operation: operation.current });
      operation.current = '';
      changed();
    } catch (error) {
      const refusal = asRefused(error);
      if (refusal.status >= 400 && refusal.status < 500) operation.current = '';
      setRefused(refusal);
    }
    setBusy(false);
  };
  return <section className="card">
    <div className="set-row">
      <div>Restart {entry.display_name}<div className="d">Ends this run and starts it again in the same folder, on {session.machine_name ?? session.machine}. What it has not saved is lost.</div></div>
      <button type="button" className="btn" disabled={busy} onClick={() => { void restart(); }}>{busy ? 'Restarting…' : 'Restart'}</button>
    </div>
    <Said refused={refused} />
  </section>;
}

function Settings({ entry, session, changed, done }: { entry: Entry; session: RuntimeSession | undefined; changed: () => void; done: () => void }) {
  const onKey = (event: KeyboardEvent<HTMLDivElement>) => {
    if (event.key === 'Escape') { event.stopPropagation(); done(); }
  };
  // Focused once, when it opens: a callback ref would take the focus back from a field on every render of the page.
  const box = useRef<HTMLDivElement>(null);
  useEffect(() => { box.current?.focus(); }, []);
  return <div className="team-settings" role="dialog" aria-label={entry.display_name + ' settings'} tabIndex={-1} onKeyDown={onKey} ref={box}>
    <div className="team-settings-head"><h2>{entry.display_name}</h2><button type="button" className="btn" onClick={() => { changed(); done(); }}>{session ? 'Back to the terminal' : 'Done'}</button></div>
    {session ? <Restart entry={entry} session={session} changed={() => { changed(); done(); }} /> : null}
    <Provisioning id={entry.id} />
    <AgentUsage agent={entry.id} />
  </div>;
}

export function Team() {
  const { pathname, search } = useLocation();
  const open = openAgents(pathname, search);
  const [revision, setRevision] = useState(0);
  const [treeShown, setTree] = useState(() => pref(TREE, 'shown'));
  const [settingsFor, setSettingsFor] = useState<string | null>(null);
  const [stopping, setStopping] = useState<string | null>(null);
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
    return <p className="team-screen why-not team-refused">Lys could not read your teams and agents: {status === 401 ? 'you are not signed in. Sign in to Lys, then reload this page.' : status ? `the service answered ${status}: ${refusal.reason}` : refusal.reason}</p>;
  }
  const { tree, sessions, me, admin } = load.data;
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
            {session
              ? <Terminal key={session.session} session={session.session} agent={entry.id} bare />
              : <div className="team-stopped"><Start key={entry.id} entry={entry} me={me} admin={admin} changed={changed} settings={() => setSettingsFor(entry.id)} /></div>}
            {session && stopping === entry.id ? <Stop entry={entry} session={session} changed={changed} done={() => setStopping(null)} /> : null}
            {inSettings ? <Settings entry={entry} session={session} changed={changed} done={() => setSettingsFor(null)} /> : null}
          </div>
          <div className="team-foot">
            {index === 0 ? <button type="button" className="icon-btn team-foot-tree" aria-pressed={shown} title={shown ? 'Hide the list of agents' : 'Show the list of agents'} onClick={toggleTree}>{shown ? '‹' : '›'}</button> : null}
            <span className="team-foot-name">{entry.display_name}</span>
            {session ? <span className="team-foot-machine">on {session.machine_name ?? session.machine}</span> : null}
            <button type="button" className="team-foot-act" aria-pressed={inSettings} onClick={() => setSettingsFor(inSettings ? null : entry.id)}>{inSettings ? session ? 'Back to the terminal' : 'Done' : 'Settings'}</button>
            {session ? <button type="button" className="team-foot-act" onClick={() => setStopping(entry.id)}>Stop</button> : null}
            {panes.length > 1 ? <a className="team-foot-act" href={addressOf(open.filter((id) => id !== entry.id))} title="Close this pane; the agent keeps running">Close</a> : null}
          </div>
        </div>;
      })}</div>
      : <div className="team-empty">{agents.length
        ? <p className="dim">Choose an agent from the list.</p>
        : <><p>You have no agents yet.</p><a className="btn primary" href="#/agents/new">Add an agent</a></>}</div>}
  </div></div>;
}
