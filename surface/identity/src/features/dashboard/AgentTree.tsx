/**
 * The person's agents as one table, grouped under their teams as a tree that folds; an agent in several teams is
 * listed under each. Each row says where each of its live sessions runs, its budget and goals, and offers Watch and
 * Stop for each session, or Start or Set up. Start and Stop open a row of their own under the row they were pressed in.
 */
import { Fragment, useState } from 'react';
import type { CSSProperties, ReactNode } from 'react';
import { useNavigate } from 'react-router';
import { api, request, useLoad } from '../../api';
import { keyable } from '../../shell/keyable';
import { pref, setPref } from '../../shell/prefs';
import type { Entry } from '../people/directory';
import type { Branch } from '../people/tree';
import type { ProvisioningAnswer } from '../provisioning/Provisioning';
import type { RuntimeSession } from '../runtime/RuntimeSessions';
import { Stop } from '../team/AgentRun';
import { Start } from '../team/Start';
import '../team/team.css';
import { refusedPart } from './contract';
import type { DashboardAgent } from './contract';
import { budgetLine, goalsLine } from './words';

/**
 * What the person asked from one row: start the agent, or stop one of its sessions. `place` is the row the press came
 * from (an agent listed under two teams has two rows, and Running now has its own), so the row it opens is under that one.
 */
export interface Asked { agent: string; what: 'start' | 'stop' | 'started'; pressed: boolean; place: string; session?: string }

/** What every row needs from the page: the one read, the person, the asked row, and the picture. */
export interface RowContext {
  rows: Map<string, DashboardAgent>;
  me: string;
  asked: Asked | null;
  ask: (next: Asked | null) => void;
  changed: () => void;
  watch: (agent: string, session: string) => void;
  /** The session the picture shows. */
  watching: string | null;
  /** Why no agent can be started while everything is stopped; null while agents may start. */
  stopped: string | null;
}

/** An agent's live sessions; none when they could not be read, which its row says by the refusal's name. */
export const liveOf = (row: DashboardAgent | undefined): RuntimeSession[] =>
  row && Array.isArray(row.sessions) ? row.sessions.filter((session) => session.shown !== 'stopped') : [];
/** Where a session runs, by the computer's name; a computer with no name is never shown by its identifier. */
export const whereOf = (session: RuntimeSession): string => 'on ' + (session.machine_name ?? 'a computer with no name');
export const entryOf = (row: DashboardAgent): Entry => ({ id: row.agent.id, display_name: row.agent.display_name, state: row.agent.state, kind: 'agent', role: null, person: null });
const FOLDED = 'you-folded';
const readFolded = () => new Set(pref(FOLDED, '').split(',').filter(Boolean));
const depth = (level: number) => ({ '--depth': level } as CSSProperties);
const COLUMNS = 5;

/** Table rows cannot be wrapped in an element, so a keyed fragment stands in. */
function FragmentRows({ children }: { children: ReactNode }) { return <>{children}</>; }

/** Start, where the agent is listed. Whether the person may approve its settings is read here, when it is first shown. Its settings have one home, the Settings tab of its own page. */
function StartHere({ entry, me, pressed, changed, close }: { entry: Entry; me: string; pressed: boolean; changed: () => void; close: () => void }) {
  const navigate = useNavigate();
  const load = useLoad(() => api.people(), 'you-start:' + entry.id);
  if (load.status === 'loading') return null;
  return <div className="you-start-here">
    <Start entry={entry} me={me} admin={load.status === 'ok' && load.data.scope === 'directory'} changed={changed} straightAway={pressed}
      settings={() => navigate('/file/' + encodeURIComponent(entry.id) + '/provisioning')} />
    <button type="button" className="you-watch" data-act="close" onClick={close}>Close</button>
  </div>;
}

/** What a stopped agent's row offers: Start when it has a program chosen, Set up when it has none, so no button says Start that cannot start. */
function RowStart({ entry, ask, place, stopped }: { entry: Entry; ask: (next: Asked) => void; place: string; stopped: string | null }) {
  const load = useLoad(() => request<ProvisioningAnswer>('/agents/' + encodeURIComponent(entry.id) + '/provisioning'), 'you-row:' + entry.id);
  if (load.status === 'loading') return null;
  const unset = load.status === 'ok' && !load.data.profile?.harness;
  return unset
    ? <a className="you-watch you-start" data-act="setup" href={'#/file/' + encodeURIComponent(entry.id) + '/provisioning'} onClick={(event) => event.stopPropagation()}>Set up</a>
    : <button type="button" className="you-watch you-start" data-act="start" disabled={stopped !== null} title={stopped ?? undefined} onClick={(event) => { event.stopPropagation(); ask({ agent: entry.id, what: 'start', pressed: true, place }); }}>Start</button>;
}

/** Watch and Stop for one live session, the same in the agents table and under Running now. */
export function RunningActs({ entry, session, context, place }: { entry: Entry; session: RuntimeSession; context: RowContext; place: string }) {
  return <span className="dash-acts" data-session={session.session}>
    <button type="button" className="you-watch" data-act="watch" aria-pressed={context.watching === session.session} title={'Watch ' + entry.display_name + ' ' + whereOf(session)}
      onClick={(event) => { event.stopPropagation(); context.watch(entry.id, session.session); }}>Watch</button>
    <button type="button" className="you-watch" data-act="stop" title={'Stop ' + entry.display_name + ' ' + whereOf(session)}
      onClick={(event) => { event.stopPropagation(); context.ask({ agent: entry.id, what: 'stop', pressed: true, place, session: session.session }); }}>Stop</button>
  </span>;
}

/** The row under the row Start or Stop was pressed in. */
export function AskedRow({ entry, place, context, columns }: { entry: Entry; place: string; context: RowContext; columns: number }) {
  const { asked, ask, me, changed } = context;
  if (asked?.agent !== entry.id || asked.place !== place) return null;
  const live = liveOf(context.rows.get(entry.id));
  const session = asked.what === 'stop' ? live.find((each) => each.session === asked.session) : undefined;
  if (asked.what === 'stop' ? !session : live.length) return null;
  // While everything is stopped no Start is offered, even in a row opened before.
  if (asked.what === 'start' && context.stopped !== null) return null;
  return <tr className="you-asked"><td colSpan={columns}>
    {session
      ? <Stop entry={entry} session={session} changed={changed} done={() => ask(null)} />
      : asked.what === 'started'
      // The start was answered; the row offers no second Start while the page reads where it is running.
      ? <p role="status" className="team-start-line">{entry.display_name} has started. Reading where it is running…</p>
      : <StartHere key={entry.id + (asked.pressed ? ':pressed' : '')} entry={entry} me={me} pressed={asked.pressed} changed={() => { ask({ agent: entry.id, what: 'started', pressed: false, place }); changed(); }} close={() => ask(null)} />}
  </td></tr>;
}

function Refusal({ name }: { name: string }) { return <small className="refusal-name why-not">{name}</small>; }

function Cell({ line }: { line: { words: string; refused: boolean } }) {
  return <td className="sec you-where">{line.refused ? <Refusal name={line.words} /> : line.words}</td>;
}

/** One agent's row where it is listed: `place` tells this listing from its others. */
function AgentRow({ entry, level, place, open, context }: { entry: Entry; level: number; place: string; open: (id: string) => void; context: RowContext }) {
  const row = context.rows.get(entry.id);
  const live = liveOf(row);
  const asked = context.asked;
  const sessionsRefused = row && refusedPart(row.sessions) ? row.sessions.refusal : null;
  const teamsRefused = row && refusedPart(row.teams) ? row.teams.refusal : null;
  return <>
    <tr data-href={'#/file/' + entry.id} {...keyable(() => open(entry.id))}>
      <td style={depth(level)} className="you-agent"><span className={'dot ' + (live.length ? 's-active' : 's-retired')} aria-label={sessionsRefused ? 'not known' : live.length ? 'running' : 'not running'} />{entry.display_name}
        {teamsRefused ? <> <span className="sec">teams:</span> <Refusal name={teamsRefused} /></> : null}</td>
      <td className="sec you-where">{sessionsRefused ? <Refusal name={sessionsRefused} />
        : live.length ? live.map((session) => <span className="dash-line" key={session.session}>{whereOf(session)}</span>)
        : entry.state === 'active' ? 'not running' : entry.state}</td>
      {row ? <><Cell line={budgetLine(row.budget, row.usage)} /><Cell line={goalsLine(row.goals)} /></> : <td colSpan={2} />}
      <td className="you-act">{sessionsRefused ? null
        : live.length ? live.map((session) => <RunningActs key={session.session} entry={entry} session={session} context={context} place={place} />)
        : asked?.agent === entry.id && asked.what !== 'stop' ? null : <RowStart entry={entry} ask={context.ask} place={place} stopped={context.stopped} />}</td>
    </tr>
    <AskedRow entry={entry} place={place} context={context} columns={COLUMNS} />
  </>;
}

function AgentRows({ branches, level, place, folded, fold, open, context }: {
  branches: Branch[]; level: number; place: string; folded: Set<string>; fold: (team: string) => void; open: (id: string) => void; context: RowContext;
}) {
  return <>{branches.map((branch, index) => {
    const away = branch.team ? folded.has(branch.team.id) : false;
    const running = branch.members.filter((member) => liveOf(context.rows.get(member.entry.id)).length).length;
    const here = place + '/' + (branch.team?.id ?? 'loose-' + index);
    return <FragmentRows key={branch.team?.id ?? 'loose-' + index}>
      {branch.team ? <tr className="you-team">
        <td colSpan={COLUMNS} style={depth(level)}>
          <button type="button" className="you-fold" aria-expanded={!away} onClick={() => fold(branch.team!.id)}>
            <span className="you-chevron" aria-hidden="true" />{branch.team.name}
            <span className="you-count">{running ? running + ' running of ' : ''}{branch.members.length}</span>
          </button>
        </td>
      </tr> : null}
      {away ? null : branch.members.map((member) => <FragmentRows key={member.entry.id}>
        <AgentRow entry={member.entry} level={level + (branch.team ? 1 : 0)} place={here + '/' + member.entry.id} open={open} context={context} />
        <AgentRows branches={member.branches} level={level + 1} place={here + '/' + member.entry.id} folded={folded} fold={fold} open={open} context={context} />
      </FragmentRows>)}
    </FragmentRows>;
  })}</>;
}

/** The agents table: its head, the tree, and the one sentence when there is no agent. */
export function AgentTree({ branches, context }: { branches: Branch[]; context: RowContext }) {
  const navigate = useNavigate();
  const [folded, setFolded] = useState(readFolded);
  const fold = (team: string) => {
    const next = new Set(folded);
    if (next.has(team)) next.delete(team); else next.add(team);
    setPref(FOLDED, [...next].join(','));
    setFolded(next);
  };
  return <table className="you-tree">
    <thead><tr><th>Agent</th><th>Running</th><th>Budget</th><th>Goals</th><th aria-label="Actions" /></tr></thead>
    <tbody>
      {context.rows.size
        ? <AgentRows branches={branches} level={0} place="tree" folded={folded} fold={fold} open={(id) => navigate('/file/' + id)} context={context} />
        : <tr><td colSpan={COLUMNS} className="dim">You have no agents.</td></tr>}
    </tbody>
  </table>;
}

/** Running now: one row for each live session, with its own Watch and Stop. */
export function RunningNow({ rows, context, first = null }: { rows: DashboardAgent[]; context: RowContext; first?: ReactNode }) {
  const running = rows.flatMap((row) => liveOf(row).map((session) => ({ entry: entryOf(row), session })));
  // An agent whose sessions could not be read is not counted as stopped: the panel names it and says why.
  const unread = rows.flatMap((row) => refusedPart(row.sessions) ? [{ entry: entryOf(row), refusal: row.sessions }] : []);
  return <table className="dash-table" aria-label="Running now">
    <tbody>{first}{running.length ? running.map(({ entry, session }) => {
      const place = 'running/' + session.session;
      return <Fragment key={session.session}>
        <tr data-running={session.session} data-agent={entry.id}>
          <td>{entry.display_name}</td><td className="sec">{whereOf(session)}</td>
          <td className="you-act"><RunningActs entry={entry} session={session} context={context} place={place} /></td>
        </tr>
        <AskedRow entry={entry} place={place} context={context} columns={3} />
      </Fragment>;
    }) : unread.length ? null : <tr className="empty"><td className="dim">Nothing is running.</td></tr>}
    {unread.map(({ entry, refusal }) => <tr key={entry.id} data-unread={entry.id}>
      <td>{entry.display_name}</td><td className="why-not" colSpan={2} title={refusal.reason}>Whether it is running could not be read. <Refusal name={refusal.refusal} /></td>
    </tr>)}</tbody>
  </table>;
}
