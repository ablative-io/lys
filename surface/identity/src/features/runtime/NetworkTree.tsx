import { useMemo, useRef, useState } from 'react';
import { useLive } from '../../api';
import type { SeatView } from '../../api';
import { Gate } from '../signin/Gate';
import { branches, readNetworkTree } from './network-tree';
import type { TreeBranch } from './network-tree';
import { NetworkTreePane, SeatStart } from './NetworkTreePane';
import './network-tree.css';

const MARKS = { 'online-idle': '●', 'online-working': '◉', offline: '○', 'not-seen': '◌', unknown: '?' };

export function NetworkBranches({ branches: all, selected, select, change }: { branches: TreeBranch[]; selected: string | null; select: (seat: string) => void; change: (seat: SeatView) => void }) {
  return <ul className="network-branches" aria-label="Network machines">{all.map(({ machine, leaves }) => <li key={machine.id} data-machine={machine.id}>
    <h2><span title={machine.name}>{machine.name}</span><small>{machine.kind}</small></h2>
    <ul>{leaves.length ? leaves.map(({ seat, name, reason }) => <li className="network-leaf" key={seat.name} data-seat={seat.name} data-state={seat.state} data-selected={selected === seat.name}>
        <a href={'#/canvas?seat=' + encodeURIComponent(seat.name)} title={name} className="network-name" data-mark={MARKS[seat.state]} data-detail={(seat.transport ?? 'not started') + ' · ' + seat.state} aria-label={name + ', ' + (seat.transport ?? 'not started') + ', ' + seat.state} aria-current={selected === seat.name ? 'true' : undefined} onClick={(event) => { event.preventDefault(); select(seat.name); }}>{name}</a>
      {seat.state === 'offline' || seat.state === 'not-seen' ? <SeatStart seat={seat} change={change} /> : null}
      {reason ? <p className="network-reason">{reason}</p> : null}
    </li>) : <li className="dim">no seats</li>}</ul>
  </li>)}</ul>;
}

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
  return <Gate load={load} title="Network tree" ok={() => <div className="network-tree" style={{ '--network-pane-width': (width ?? paneWidth) + 'px' } as React.CSSProperties}>
    <div className="network-list"><NetworkBranches branches={all} selected={chosen?.seat.name ?? null} select={select} change={change} />
      {!all.length ? <p className="note">No active machine was returned.</p> : null}
    </div>
    <div className="network-divider" role="separator" aria-label="Terminal pane width" aria-orientation="vertical" tabIndex={0} aria-valuenow={width ?? paneWidth} aria-valuemin={280} aria-valuemax={1200}
      onKeyDown={(event) => { if (event.key === 'ArrowLeft' || event.key === 'ArrowRight') { event.preventDefault(); resize(Math.max(280, Math.min(1200, paneWidth + (event.key === 'ArrowLeft' ? 24 : -24)))); } }}
      onPointerDown={(event) => { drag.current = event.clientX; event.currentTarget.setPointerCapture(event.pointerId); }}
      onPointerMove={(event) => { if (drag.current !== null) setWidth(Math.max(280, Math.min(1200, paneWidth + drag.current - event.clientX))); }}
      onPointerUp={() => { if (drag.current !== null && width !== null) resize(width); drag.current = null; setWidth(null); }} onPointerCancel={() => { drag.current = null; setWidth(null); }} />
    <div className="network-pane">{chosen ? <NetworkTreePane key={chosen.seat.name + ':' + (chosen.seat.session ?? '')} {...chosen} change={change} /> : <p className="note" role="status">{selected ? 'The selected seat was not returned on an active machine.' : 'Select an agent to watch its terminal.'}</p>}</div>
  </div>} />;
}
