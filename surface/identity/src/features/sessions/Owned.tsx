/** AGENTS-004: the seats this runner's owners hold, live and unreachable, as the runner proved them at the kernel. */
import { send, useLoad } from '../../api';
import { clockMs } from '../file/time';
import { Gate } from '../signin/Gate';

/** crates/lys-identity-server/src/seat_supervision.rs: OwnedSeatView and OwnedSeats. */
export interface OwnedSeatView {
  seat: string;
  session: string;
  conversation: string;
  generation: number;
  owner_pid: number;
  owner_start: string;
  established_at: number;
  unreachable?: { why: 'exited' | 'pid_reused' | 'unproved'; found?: string; reason?: string } | null;
}
/** GET /seats/owned: every owner the runner started, and whether the runner was read. */
export interface OwnedSeats { owners: OwnedSeatView[]; runner: 'read' | 'unknown'; reason?: string }

function reach(view: OwnedSeatView): string {
  if (!view.unreachable) return 'Live: proved at the kernel';
  switch (view.unreachable.why) {
    case 'exited': return 'Unreachable: no process has the owner\'s pid';
    case 'pid_reused': return 'Unreachable: the pid belongs to another process (' + (view.unreachable.found ?? '') + ')';
    default: return 'Unreachable: the kernel could not be asked (' + (view.unreachable.reason ?? '') + ')';
  }
}

export function OwnedList() {
  const load = useLoad(() => send<OwnedSeats>('GET', '/seats/owned'), '/seats/owned');
  return <Gate load={load} title="the owned seats" renderError={(error) => <p role="alert" className="why-not">The owned seats could not be read: {String(error)}</p>} ok={(view) => <>
    {view.runner === 'unknown' ? <p role="status" className="note">The runner could not be read{view.reason ? ': ' + view.reason : ''}. Every owner below is as last recorded.</p> : null}
    {view.owners.length ? <table>
      <thead><tr><th>Seat</th><th>Session</th><th>Generation</th><th>Owner</th><th>Established</th></tr></thead>
      <tbody>{view.owners.map((owner) => <tr key={owner.session}>
        <td>{owner.seat}</td><td>{owner.session}<p className="note">Conversation {owner.conversation}</p></td><td>{owner.generation}</td>
        <td>pid {owner.owner_pid}<p className="note">{reach(owner)}</p></td><td>{clockMs(owner.established_at)}</td>
      </tr>)}</tbody>
    </table> : <p className="note">This runner has started no seat owner.</p>}
  </> } />;
}
