/** AGENTS-004: the seats this runner's owners hold, live and unreachable, as the runner proved them at the kernel. */
import { send, useLoad } from '../../api';
import { clockMs } from '../file/time';
import { Gate } from '../signin/Gate';
import { ErrorWords } from '../people/Words';
import { TableScroll } from '../../shell/Listing';

/** crates/lys-identity-server/src/seat_supervision.rs: OwnedSeatView and OwnedSeats. */
export interface OwnedSeatView {
  seat: string;
  session: string;
  conversation: string;
  generation: number;
  build: string;
  owner_pid: number;
  owner_start: string;
  established_at: number;
  unreachable?: { why: 'exited' | 'pid_reused' | 'unproved'; found?: string; reason?: string } | null;
}
/** GET /seats/owned: every owner the runner started, and whether the runner was read. */
export interface OwnedSeats { owners: OwnedSeatView[]; runner: 'read' | 'unknown'; reason?: string }

function reach(view: OwnedSeatView): string {
  if (!view.unreachable) return 'Running';
  switch (view.unreachable.why) {
    case 'exited': return 'Ended: its process is gone';
    case 'pid_reused': return 'Ended: its process number now belongs to another program' + (view.unreachable.found ? ' (' + view.unreachable.found + ')' : '');
    default: return 'Not known: the computer could not be asked' + (view.unreachable.reason ? ' (' + view.unreachable.reason + ')' : '');
  }
}

export function OwnedList() {
  const load = useLoad(() => send<OwnedSeats>('GET', '/seats/owned'), '/seats/owned');
  return <Gate load={load} title="the owned seats" renderError={(error) => <><p className="why-not">The seat processes could not be read.</p><ErrorWords problem={error} /></>} ok={(view) => <>
    {view.runner === 'unknown' ? <p role="status" className="note">Lys could not reach the runner{view.reason ? ': ' + view.reason : ''}. Each process below is as last recorded.</p> : null}
    {view.owners.length ? <TableScroll label="Seat processes"><table>
      <thead><tr><th>Seat</th><th>Session</th><th>Start number</th><th>Lys build</th><th>Process</th><th>Since</th></tr></thead>
      <tbody>{view.owners.map((owner) => <tr key={owner.session}>
        <td>{owner.seat}</td><td>{owner.session}<p className="note">Conversation {owner.conversation}</p></td><td>{owner.generation}</td><td>{owner.build}</td>
        <td>Process {owner.owner_pid}<p className="note">{reach(owner)}</p></td><td>{clockMs(owner.established_at)}</td>
      </tr>)}</tbody>
    </table></TableScroll> : <p className="note">No seat process has been started on this computer.</p>}
  </> } />;
}
