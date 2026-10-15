import { useEffect, useMemo, useRef, useState } from 'react';
import { useLive } from '../../api';
import type { SeatView } from '../../api';
import { Gate } from '../signin/Gate';
import { AGENT_CHOICE, branches, readNetworkTree, readRunning, SESSION_CHOICE, useAgentNames } from './network-tree';
import { Terminal } from './Terminal';
import { said } from './canvas-kept';
import type { TreeAgent, TreeBranch } from './network-tree';
import { NetworkTreePane } from './NetworkTreePane';
import { PANE_WIDTH } from './canvas-kept';
import { fifthOf } from './canvas-operations';
import { NetworkAgentStart } from './NetworkAgentPane';
import { STATE_WORDS } from '../sessions/Seats';
import './network-tree.css';

const MARKS = { 'online-idle': '●', 'online-working': '◉', offline: '○', 'not-seen': '◌', unknown: '?' };

/** A seat's standing in a few words. */
const detail = (seat: SeatView): string => STATE_WORDS[seat.state] ?? 'State not known';

/** The one seat row: the state dot, the name, and everything else in the title. */
function Leaf({ seat, name, reason, label, selected, select, agent }: { seat: SeatView; name: string; reason: string | null; label: string; selected: string | null; select: (seat: string) => void; agent?: string }) {
  return <li className="network-leaf" data-agent={agent} data-seat={seat.name} data-state={seat.state} data-selected={selected === seat.name} role="none">
    <a href={'#/canvas?seat=' + encodeURIComponent(seat.name)} role="treeitem" title={name + ' · seat ' + seat.name + ' · ' + detail(seat) + (reason ? ' · ' + reason : '')} className="network-name" data-mark={MARKS[seat.state]} aria-label={name + ', seat ' + seat.name + ', ' + detail(seat)} aria-current={selected === seat.name ? 'true' : undefined} aria-selected={selected === seat.name} onClick={(event) => { event.preventDefault(); select(seat.name); }}>{label}</a>
  </li>;
}

/**
 * An agent's row. One seat: the agent is the row, pressed to choose that seat. Several: a fold named for the agent
 * with a row per seat. None: the agent's name alone; running when it has a live session on the computer outside any
 * seat, and then pressed to watch that session. While the live sessions have not been read, or where a runner did
 * not answer, the row says so in its title.
 */
function AgentRow({ agent, selected, select, unsurveyed }: { agent: TreeAgent; selected: string | null; select: (seat: string) => void; unsurveyed: string | null }) {
  const title = agent.reason ? agent.name + ' · ' + agent.reason : agent.name;
  if (agent.leaves.length === 1) {
    const [leaf] = agent.leaves;
    return <Leaf {...leaf} agent={agent.id} label={agent.name} selected={selected} select={select} />;
  }
  if (!agent.leaves.length) {
    const [session] = agent.running;
    if (session) {
      const choice = SESSION_CHOICE + session.session;
      return <li className="network-leaf network-seatless" data-agent={agent.id} data-agent-only={agent.id} data-session={session.session} data-state="online-working" data-selected={selected === choice} role="none">
        <a href={'#/canvas?seat=' + encodeURIComponent(choice)} role="treeitem" className="network-name" data-mark={MARKS['online-working']} title={title + ' · running outside a seat · session ' + session.session + (agent.running.length > 1 ? ' and ' + (agent.running.length - 1) + ' more' : '')} aria-label={agent.name + ', running, session ' + session.session} aria-current={selected === choice ? 'true' : undefined} aria-selected={selected === choice} onClick={(event) => { event.preventDefault(); select(choice); }}>{agent.name}</a>
      </li>;
    }
    const choice = AGENT_CHOICE + agent.id;
    return <li className="network-leaf network-seatless" data-agent={agent.id} data-agent-only={agent.id} data-state="not-seen" data-selected={selected === choice} role="none">
      <a href={'#/canvas?seat=' + encodeURIComponent(choice)} role="treeitem" className="network-name" data-mark={MARKS['not-seen']} title={title + ' · no seat on this computer' + (unsurveyed ? ' · ' + unsurveyed : '')} aria-label={agent.name + ', not running'} aria-current={selected === choice ? 'true' : undefined} aria-selected={selected === choice} onClick={(event) => { event.preventDefault(); select(choice); }}>{agent.name}</a>
    </li>;
  }
  const choice = AGENT_CHOICE + agent.id;
  return <li data-agent={agent.id} role="none"><details open><summary role="treeitem" className="network-fold network-agent" title={title + ' · ' + agent.leaves.length + ' seats'} data-selected={selected === choice} aria-selected={selected === choice} onClick={() => select(choice)}><span className="network-fold-name">{agent.name}</span></summary>
    <ul role="group">{agent.leaves.map((leaf) => <Leaf key={leaf.seat.name} {...leaf} label={leaf.seat.name} selected={selected} select={select} />)}</ul>
  </details></li>;
}

/**
 * The network as a tree, the way an editor shows files: each computer, under it every agent it may run, once each,
 * and under an agent with several seats its seats. Names, a chevron and a state dot, nothing else in it: a seat is
 * chosen by pressing its row, and everything about it (its standing, its reasons, Start when it is not running) is in
 * the terminal space beside the tree. The folds are the browser's own disclosure, so Enter and Space work on them as
 * everywhere, and the arrow keys walk the rows (see `walk`). What a row cannot show is in its title.
 */
export function NetworkBranches({ branches: all, selected, select, unsurveyed = null }: { branches: TreeBranch[]; selected: string | null; select: (seat: string) => void; unsurveyed?: string | null }) {
  return <ul className="network-branches" role="tree" aria-label="Network machines" onKeyDown={walk}>{all.map(({ machine, agents }) => <li key={machine.id} data-machine={machine.id} role="none">
    <details open><summary role="treeitem" className="network-fold network-machine" title={machine.name + ' · ' + machine.kind}><span className="network-fold-name">{machine.name}</span></summary>
      <ul role="group">{agents.length ? agents.map((agent) => <AgentRow key={agent.id} agent={agent} selected={selected} select={select} unsurveyed={unsurveyed} />) : <li className="dim network-empty" role="none">No agent may start here yet.</li>}</ul>
    </details>
  </li>)}</ul>;
}

/** The arrow keys on the tree: up and down walk the rows that are shown, left folds the row's branch, right unfolds it. */
function walk(event: React.KeyboardEvent<HTMLUListElement>) {
  const row = event.target as HTMLElement;
  if (!row.matches('[role="treeitem"]')) return;
  if (event.key === 'ArrowDown' || event.key === 'ArrowUp') {
    const rows = [...event.currentTarget.querySelectorAll<HTMLElement>('[role="treeitem"]')].filter((each) => each.offsetParent !== null);
    const next = rows[rows.indexOf(row) + (event.key === 'ArrowDown' ? 1 : -1)];
    if (next) { event.preventDefault(); next.focus(); }
    return;
  }
  if (event.key === 'ArrowLeft' || event.key === 'ArrowRight') {
    const fold = row.tagName === 'SUMMARY' ? row.parentElement as HTMLDetailsElement : null;
    if (event.key === 'ArrowRight' && fold && !fold.open) { event.preventDefault(); fold.open = true; }
    else if (event.key === 'ArrowLeft' && fold?.open) { event.preventDefault(); fold.open = false; }
    else if (event.key === 'ArrowLeft') { const above = row.closest('ul')?.parentElement?.querySelector<HTMLElement>(':scope > summary'); if (above) { event.preventDefault(); above.focus(); } }
  }
}

const within = (width: number): number => Math.round(Math.max(PANE_WIDTH.least, Math.min(PANE_WIDTH.most, width)));
/** A fifth of the window, kept within what the service keeps, while the person has not set a width of their own. */
const FIFTH = 'clamp(' + PANE_WIDTH.least + 'px, 20vw, ' + PANE_WIDTH.most + 'px)';

/**
 * The Network view: the tree down the left, a fifth of the window until the person drags or keys it to the width they
 * want (kept by the service), and the chosen row's terminal filling the rest. The tree is drawn the moment the
 * computers and the seats answer; agents' names and the live sessions fill in as they arrive.
 */
export function NetworkTree({ selected, select, paneWidth, resize, agent }: { selected: string | null; select: (seat: string) => void; paneWidth: number | null; resize: (width: number) => void; agent?: string }) {
  const load = useLive(readNetworkTree, 'operations-network-tree');
  const named = useAgentNames(load.status === 'ok' ? load.data : null);
  // The live sessions arrive on their own; the tree never waits for them.
  const running = useLive(readRunning, 'operations-network-running');
  const unsurveyed = running.status === 'ok' ? running.data.unanswered : running.status === 'refused' ? 'Running sessions could not be read: ' + said(running.refused) : 'Running sessions are still being read.';
  const [changes, setChanges] = useState<Record<string, SeatView>>({});
  const [width, setWidth] = useState<number | null>(null);
  const drag = useRef<number | null>(null);
  const watched = useRef<{ choice: string; agent: string } | null>(null);
  const all = useMemo(() => load.status === 'ok' ? branches(load.data, { ...load.data, seats: load.data.seats.map((seat) => {
    const changed = changes[seat.name];
    return changed && (seat.revision === undefined || changed.revision > seat.revision) ? changed : seat;
  }) }, named.names, named.reasons, running.status === 'ok' ? running.data.sessions : []) : [], [load, changes, running, named]);
  // An agent's own row: the agent on the computer it was chosen on, with its seats there and its live sessions.
  const agentChoice = selected?.startsWith(AGENT_CHOICE) ? all.flatMap((branch) => branch.agents.map((each) => ({ each, machine: branch.machine }))).find(({ each }) => AGENT_CHOICE + each.id === selected) ?? null : null;
  // Chosen by its seat, or by its agent: the seat that is running, else its first, which offers Start.
  const chosen = agentChoice ? agentChoice.each.leaves.find((leaf) => leaf.seat.state === 'online-idle' || leaf.seat.state === 'online-working') ?? agentChoice.each.leaves[0]
    : all.flatMap((branch) => branch.leaves).find((leaf) => selected ? leaf.seat.name === selected : agent && leaf.seat.agent === agent);
  const agentLive = agentChoice && !chosen ? agentChoice.each.running[0] ?? null : null;
  // A seatless agent's live session, chosen by its row: watched through the session's own terminal.
  const session = selected?.startsWith(SESSION_CHOICE) ? all.flatMap((branch) => branch.agents.flatMap((each) => each.running.map((live) => ({ live, agent: each, machine: branch.machine })))).find((each) => SESSION_CHOICE + each.live.session === selected) ?? null : null;
  useEffect(() => {
    if (session && selected) watched.current = { choice: selected, agent: session.agent.id };
    else if (running.status === 'ok' && load.status === 'ok' && !running.data.unanswered && watched.current?.choice === selected
      && all.some((branch) => branch.agents.some((each) => each.id === watched.current?.agent))) {
      select(AGENT_CHOICE + watched.current.agent);
    }
  }, [session, selected, running, load, all, select]);
  const change = (seat: SeatView) => setChanges((before) => ({ ...before, [seat.name]: seat }));
  const base = paneWidth ?? fifthOf(window.innerWidth);
  const shown = within(width ?? base);
  return <Gate load={load} title="Network tree" ok={() => <div className="network-tree" style={{ '--network-pane-width': width === null && paneWidth === null ? FIFTH : shown + 'px' } as React.CSSProperties}>
    <aside className="network-list" aria-label="Network tree"><NetworkBranches branches={all} selected={session || agentChoice ? selected : chosen?.seat.name ?? null} select={select} unsurveyed={unsurveyed} />
      {!all.length ? <p className="note">No computer is in use yet. <a href="#/network?add=computer">Add a computer</a> to start agents on it.</p> : null}
    </aside>
    <div className="network-divider" role="separator" aria-label="Network tree width" aria-orientation="vertical" tabIndex={0} aria-valuenow={shown} aria-valuemin={PANE_WIDTH.least} aria-valuemax={PANE_WIDTH.most}
      onKeyDown={(event) => { if (event.key === 'ArrowLeft' || event.key === 'ArrowRight') { event.preventDefault(); resize(within(base + (event.key === 'ArrowRight' ? 24 : -24))); } }}
      onPointerDown={(event) => { drag.current = event.clientX; event.currentTarget.setPointerCapture(event.pointerId); }}
      onPointerMove={(event) => { if (drag.current !== null) setWidth(within(base + event.clientX - drag.current)); }}
      onPointerUp={() => { if (drag.current !== null && width !== null) resize(within(width)); drag.current = null; setWidth(null); }} onPointerCancel={() => { drag.current = null; setWidth(null); }} />
    <div className="network-pane">{session ? <section className="terminal network-terminal" aria-label={'Terminal of ' + session.agent.name}><Terminal key={session.live.session} session={session.live.session} agent={session.agent.id} machine={session.machine.name} /></section>
      : agentLive && agentChoice ? <section className="terminal network-terminal" aria-label={'Terminal of ' + agentChoice.each.name}><Terminal key={agentLive.session} session={agentLive.session} agent={agentChoice.each.id} machine={agentChoice.machine.name} /></section>
      : agentChoice && !chosen ? <NetworkAgentStart key={agentChoice.each.id} agent={agentChoice.each.id} name={agentChoice.each.name} reason={agentChoice.each.reason} />
      : chosen ? <NetworkTreePane key={chosen.seat.name + ':' + (chosen.seat.session ?? '')} {...chosen} change={change} />
      : <p className="note" role="status">{selected?.startsWith(SESSION_CHOICE) ? running.status === 'ok' ? 'That session is no longer running. Choose another agent in the list.' : 'Reading the running sessions…' : selected ? 'That seat is no longer listed. Choose another in the list.' : 'Choose an agent in the list to watch its terminal.'}</p>}</div>
  </div>} />;
}
