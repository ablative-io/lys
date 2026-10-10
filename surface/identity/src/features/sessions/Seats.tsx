/**
 * The seats (AGENTS-002 R2, R6): each registered seat with its state as its runner knows it (the process it holds, the
 * harness session bound, the last signal seen), its Start, Stop and Restart, and a send box for each running seat. A
 * runner that cannot be read is named, and its seats are unknown, never offline and never counted as zero. The sessions
 * a runner holds for no seat are listed as seen but unregistered. No terminal workspace program is consulted.
 */
import { useRef, useState } from 'react';
import { Refused, operationId, seats, useLive } from '../../api';
import type { SeatList, SeatState, SeatView, UnregisteredSession } from '../../api';
import { refreshLive } from '../../live';
import { Gate } from '../signin/Gate';
import { Act } from '../../shell/Act';
import { clockMs } from '../file/time';

const STATE_WORDS: Record<SeatState, string> = {
  'online-working': 'Online, working',
  'online-idle': 'Online, idle',
  offline: 'Offline',
  'not-seen': 'Not seen: registered, with no live session',
  unknown: 'Unknown: its runner could not be read',
};

const running = (seat: SeatView) => seat.state === 'online-working' || seat.state === 'online-idle';

const refusalOf = (error: unknown): Refused => error instanceof Refused ? error : new Refused(0, { refusal: 'Unanswered', reason: String(error) });

/** A refusal said by its name and the service's own words. */
function Refusal({ refused }: { refused: Refused }) {
  return <p className="why-not" role="alert"><b>{refused.refusal.refusal}</b> <span className="sec">{refused.refusal.reason}</span></p>;
}

/** A read that was refused, said in place: what could not be read, the refusal's name and its words. Nothing is shown as empty. */
function Unread({ refused, what }: { refused: Refused; what: string }) {
  return <p className="why-not">{what} could not be read. <b>{refused.refusal.refusal}</b> <span className="sec">{refused.refusal.reason}</span></p>;
}

/** A refusal by name settles a change; an unreachable or unreadable answer leaves its outcome unknown, so its operation is kept for the retry. */
const settled = (refused: Refused) => refused.status >= 400 && refused.status < 500;

export function Seats() {
  const list = useLive(seats.list, 'seats');
  const unregistered = useLive(seats.unregistered, 'seats-unregistered');
  return <section className="card seats" aria-label="Seats">
    <h2>Seats</h2>
    <Gate load={list} title="the seats" renderError={(refused) => <Unread refused={refused} what="The seats" />} ok={(answer) => <SeatTable answer={answer} />} />
    <h3>Seen but unregistered</h3>
    <Gate load={unregistered} title="the sessions held for no seat" renderError={(refused) => <Unread refused={refused} what="The sessions held for no seat" />} ok={(sessions) => <Unregistered sessions={sessions} />} />
  </section>;
}

function SeatTable({ answer }: { answer: SeatList }) {
  return <>
    {answer.runner === 'unknown' ? <p className="why-not" role="status">
      <b>Runner unknown</b> <span className="sec">{answer.reason ?? 'The service gave no reason.'}</span> Every state below is unknown: nothing is known to be online or offline.
    </p> : null}
    {answer.seats.length ? <table>
      <thead><tr><th>Seat</th><th>Agent</th><th>Computer</th><th>State</th><th>Session</th><th>Acts</th></tr></thead>
      <tbody>{answer.seats.map((seat) => <SeatRow key={seat.name} seat={seat} />)}</tbody>
    </table> : <p className="note">No seat is registered. A seat is recorded with <span className="mono">lys seat add</span>.</p>}
  </>;
}

type Doing = 'start' | 'stop' | 'restart';
const DOING_WORDS: Record<Doing, { word: string; symbol: 'start' | 'stop' | 'again' }> = {
  start: { word: 'Start', symbol: 'start' },
  stop: { word: 'Stop', symbol: 'stop' },
  restart: { word: 'Restart', symbol: 'again' },
};

function SeatRow({ seat }: { seat: SeatView }) {
  const [busy, setBusy] = useState(false);
  const [refusal, setRefusal] = useState<{ doing: Doing; refused: Refused } | null>(null);
  const [done, setDone] = useState('');
  const attempt = useRef<{ doing: Doing; force: boolean; operation: string } | null>(null);
  const sending = useRef(false);
  const act = async (doing: Doing, force: boolean) => {
    if (sending.current) return;
    if (!attempt.current || attempt.current.doing !== doing || attempt.current.force !== force) attempt.current = { doing, force, operation: operationId() };
    const { operation } = attempt.current;
    sending.current = true; setBusy(true); setRefusal(null); setDone('');
    try {
      if (doing === 'start') {
        const answer = await seats.start(seat.name, operation);
        setDone('Started as session ' + answer.session + '.');
      } else if (doing === 'stop') {
        const answer = await seats.stop(seat.name, operation, force);
        setDone(answer.ended ? 'Session ' + answer.session + ' ended.' : 'Stop asked of session ' + answer.session + '; its runner has not yet confirmed it ended.');
      } else {
        const answer = await seats.restart(seat.name, operation, force);
        setDone('Restarted as session ' + answer.session + '.');
      }
      attempt.current = null;
      refreshLive();
    } catch (error) {
      const refused = refusalOf(error);
      if (settled(refused)) attempt.current = null;
      setRefusal({ doing, refused });
    } finally { sending.current = false; setBusy(false); }
  };
  const midTurn = refusal && refusal.doing !== 'start' && refusal.refused.refusal.refusal === 'seat_turn_in_progress' ? refusal.doing : null;
  return <tr data-seat={seat.name} data-state={seat.state}>
    <td>{seat.name}</td>
    <td className="mono">{seat.agent}</td>
    <td>{seat.machine}</td>
    <td>{STATE_WORDS[seat.state]}<p className="note">{seat.last_signal_at === undefined || seat.last_signal_at === null ? 'No signal seen' : 'Last signal ' + clockMs(seat.last_signal_at)}</p></td>
    <td>{seat.session ? <span className="mono">{seat.session}</span> : <span className="dim">None</span>}</td>
    <td>
      {(['start', 'stop', 'restart'] as const).map((doing) => <Act key={doing} symbol={DOING_WORDS[doing].symbol} name={DOING_WORDS[doing].word + ' ' + seat.name} word={DOING_WORDS[doing].word} className="small" disabled={busy} onClick={() => { void act(doing, false); }} />)}
      {midTurn ? <p className="why-not">A turn is in progress. {midTurn === 'stop' ? 'Stopping' : 'Restarting'} anyway ends it mid-turn.
        <Act symbol={DOING_WORDS[midTurn].symbol} name={DOING_WORDS[midTurn].word + ' ' + seat.name + ' anyway'} word={DOING_WORDS[midTurn].word + ' anyway'} tone="danger" className="small" disabled={busy} onClick={() => { void act(midTurn, true); }} />
      </p> : null}
      {refusal ? <Refusal refused={refusal.refused} /> : null}
      {done ? <p role="status">{done}</p> : null}
      {running(seat) ? <SendBox seat={seat} /> : null}
    </td>
  </tr>;
}

/** A message to a running seat, delivered to its managed session as a user turn; never typed keys. */
function SendBox({ seat }: { seat: SeatView }) {
  const [text, setText] = useState('');
  const [busy, setBusy] = useState(false);
  const [refused, setRefused] = useState<Refused | null>(null);
  const [delivered, setDelivered] = useState('');
  const attempt = useRef<{ text: string; operation: string } | null>(null);
  const sending = useRef(false);
  const send = async () => {
    if (sending.current || !text) return;
    if (!attempt.current || attempt.current.text !== text) attempt.current = { text, operation: operationId() };
    const { operation } = attempt.current;
    sending.current = true; setBusy(true); setRefused(null); setDelivered('');
    try {
      const answer = await seats.send(seat.name, operation, text);
      attempt.current = null;
      setText('');
      setDelivered('Delivered to session ' + answer.session + ' as a user turn.');
    } catch (error) {
      const refusal = refusalOf(error);
      if (settled(refusal)) attempt.current = null;
      setRefused(refusal);
    } finally { sending.current = false; setBusy(false); }
  };
  return <form className="seat-send" aria-label={'Send to ' + seat.name} onSubmit={(event) => { event.preventDefault(); void send(); }}>
    <input name="text" aria-label={'Message for ' + seat.name} placeholder={'Message for ' + seat.name} value={text} onChange={(event) => setText(event.target.value)} />
    <Act symbol="send" name={'Send to ' + seat.name} word="Send" tone="primary" className="small" type="submit" disabled={busy || !text} />
    {delivered ? <p role="status">{delivered}</p> : null}
    {refused ? <Refusal refused={refused} /> : null}
  </form>;
}

function Unregistered({ sessions }: { sessions: UnregisteredSession[] }) {
  return sessions.length ? <table aria-label="Seen but unregistered">
    <thead><tr><th>Session</th><th>Computer</th><th>Agent</th><th>Process</th><th>Started</th></tr></thead>
    <tbody>{sessions.map((entry) => <tr key={entry.session} data-unregistered={entry.session}>
      <td className="mono">{entry.session}</td>
      <td>{entry.machine}</td>
      <td>{entry.agent ? <span className="mono">{entry.agent}</span> : <span className="dim">No agent named</span>}</td>
      <td className="mono">{entry.pid}</td>
      <td>{clockMs(entry.started_at)}</td>
    </tr>)}</tbody>
  </table> : <p className="note">No runner holds a session for no seat.</p>;
}
