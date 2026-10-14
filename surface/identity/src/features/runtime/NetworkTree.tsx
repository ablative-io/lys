import { useMemo, useRef, useState } from 'react';
import { useLive } from '../../api';
import type { SeatView } from '../../api';
import { Gate } from '../signin/Gate';
import { branches, readNetworkTree } from './network-tree';
import type { TreeBranch } from './network-tree';
import { NetworkTreePane, SeatStart } from './NetworkTreePane';
import { PANE_WIDTH } from './canvas-kept';
import './network-tree.css';

const MARKS = { 'online-idle': '●', 'online-working': '◉', offline: '○', 'not-seen': '◌', unknown: '?' };

/** A seat's standing in a few words: its terminal's kind and its state. */
const detail = (seat: SeatView): string => (seat.transport ?? 'not started') + ' · ' + seat.state;

/**
 * The network as a tree: each computer, under it each agent that has a seat there, under that the seats. A computer
 * and an agent fold; a seat is chosen. The folds are the browser's own disclosure, so Enter and Space work on them
 * as they do everywhere, and the arrow keys walk the rows (see `walk`).
 */
export function NetworkBranches({ branches: all, selected, select, change }: { branches: TreeBranch[]; selected: string | null; select: (seat: string) => void; change: (seat: SeatView) => void }) {
  return <ul className="network-branches" role="tree" aria-label="Network machines" onKeyDown={walk}>{all.map(({ machine, agents }) => <li key={machine.id} data-machine={machine.id} role="none">
    <details open><summary role="treeitem" className="network-fold network-machine" title={machine.name + ' · ' + machine.kind}><span className="network-fold-name">{machine.name}</span><small>{machine.kind}</small></summary>
      <ul role="group">{agents.length ? agents.map((agent) => <li key={agent.id} data-agent={agent.id} role="none">
        <details open><summary role="treeitem" className="network-fold network-agent" title={agent.name}><span className="network-fold-name">{agent.name}</span><small>{agent.leaves.length === 1 ? '1 seat' : agent.leaves.length + ' seats'}</small></summary>
          {agent.reason ? <p className="network-reason">{agent.reason}</p> : null}
          <ul role="group">{agent.leaves.map(({ seat, name, reason }) => <li className="network-leaf" key={seat.name} data-seat={seat.name} data-state={seat.state} data-selected={selected === seat.name} role="none">
            <a href={'#/canvas?seat=' + encodeURIComponent(seat.name)} role="treeitem" title={name + ' · ' + detail(seat)} className="network-name" data-mark={MARKS[seat.state]} aria-label={name + ', seat ' + seat.name + ', ' + detail(seat)} aria-current={selected === seat.name ? 'true' : undefined} aria-selected={selected === seat.name} onClick={(event) => { event.preventDefault(); select(seat.name); }}>{seat.name}</a>
            {seat.state === 'offline' || seat.state === 'not-seen' ? <SeatStart seat={seat} change={change} /> : null}
            {reason && reason !== agent.reason ? <p className="network-reason">{reason}</p> : null}
          </li>)}</ul>
        </details>
      </li>) : <li className="dim" role="none">no seats</li>}</ul>
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

/**
 * The Network view: the tree down the left, about a fifth of the page and dragged or keyed to the width the person
 * wants (kept by the service), and the chosen seat's terminal filling the rest.
 */
export function NetworkTree({ selected, select, paneWidth, resize, agent }: { selected: string | null; select: (seat: string) => void; paneWidth: number; resize: (width: number) => void; agent?: string }) {
  const load = useLive(readNetworkTree, 'operations-network-tree');
  const [changes, setChanges] = useState<Record<string, SeatView>>({});
  const [width, setWidth] = useState<number | null>(null);
  const drag = useRef<number | null>(null);
  const all = useMemo(() => load.status === 'ok' ? branches(load.data, { ...load.data, seats: load.data.seats.map((seat) => {
    const changed = changes[seat.name];
    return changed && (seat.revision === undefined || changed.revision > seat.revision) ? changed : seat;
  }) }, load.data.names, load.data.nameReasons) : [], [load, changes]);
  const chosen = all.flatMap((branch) => branch.leaves).find((leaf) => selected ? leaf.seat.name === selected : agent && leaf.seat.agent === agent);
  const change = (seat: SeatView) => setChanges((before) => ({ ...before, [seat.name]: seat }));
  const shown = within(width ?? paneWidth);
  return <Gate load={load} title="Network tree" ok={() => <div className="network-tree" style={{ '--network-pane-width': shown + 'px' } as React.CSSProperties}>
    <aside className="network-list" aria-label="Network tree"><NetworkBranches branches={all} selected={chosen?.seat.name ?? null} select={select} change={change} />
      {!all.length ? <p className="note">No active machine was returned.</p> : null}
    </aside>
    <div className="network-divider" role="separator" aria-label="Network tree width" aria-orientation="vertical" tabIndex={0} aria-valuenow={shown} aria-valuemin={PANE_WIDTH.least} aria-valuemax={PANE_WIDTH.most}
      onKeyDown={(event) => { if (event.key === 'ArrowLeft' || event.key === 'ArrowRight') { event.preventDefault(); resize(within(paneWidth + (event.key === 'ArrowRight' ? 24 : -24))); } }}
      onPointerDown={(event) => { drag.current = event.clientX; event.currentTarget.setPointerCapture(event.pointerId); }}
      onPointerMove={(event) => { if (drag.current !== null) setWidth(within(paneWidth + event.clientX - drag.current)); }}
      onPointerUp={() => { if (drag.current !== null && width !== null) resize(within(width)); drag.current = null; setWidth(null); }} onPointerCancel={() => { drag.current = null; setWidth(null); }} />
    <div className="network-pane">{chosen ? <NetworkTreePane key={chosen.seat.name + ':' + (chosen.seat.session ?? '')} {...chosen} change={change} /> : <p className="note" role="status">{selected ? 'The selected seat was not returned on an active machine.' : 'Choose a seat in the tree to watch its terminal.'}</p>}</div>
  </div>} />;
}
