import { useEffect, useState } from 'react';
import { request } from '../../api';
import type { SeatView } from '../../api';
import { GpuTerminal } from './GpuTerminal';
import type { SessionEnd } from './Terminal';
import { online, useSeatAction } from './network-tree';
import { STATE_WORDS } from '../sessions/Seats';
import { ErrorWords, problemWords } from '../people/Words';
import './terminal.css';
import { Act } from '../../shell/Act';
import { RUN_WORDS, RunConfirm } from '../run/RunConfirm';

interface Attached { seat: SeatView; session: string; cursor: number; ended: boolean; lines: { at: number; kind: string; text: string }[] }

function attachAnswer(answer: Attached, seat: SeatView, cursor: number | undefined): Attached {
  if (!answer || answer.seat?.name !== seat.name || answer.session !== seat.session || !Number.isSafeInteger(answer.cursor)
    || answer.cursor < (cursor ?? 0) || typeof answer.ended !== 'boolean' || !Array.isArray(answer.lines)
    || answer.lines.some((line) => typeof line.text !== 'string' || typeof line.kind !== 'string' || !Number.isSafeInteger(line.at))
    || (!answer.ended && answer.cursor === cursor)) throw new Error('Lys answered, but the agent\'s output could not be read. Choose the seat again to reconnect.');
  return answer;
}

function ManagedTerminal({ seat, end, failure }: { seat: SeatView; end: () => void; failure: (problem: unknown) => void }) {
  const [lines, setLines] = useState<Attached['lines']>([]);
  useEffect(() => {
    const controller = new AbortController();
    const follow = async () => {
      let cursor: number | undefined;
      while (!controller.signal.aborted) {
        const answer = attachAnswer(await request<Attached>('/seats/' + encodeURIComponent(seat.name) + '/attach', { ...(cursor === undefined ? {} : { cursor }), follow: true }, 'POST', controller.signal), seat, cursor);
        if (controller.signal.aborted) return;
        cursor = answer.cursor;
        setLines((before) => [...before, ...answer.lines]);
        if (answer.ended) { end(); return; }
      }
    };
    void follow().catch((error: unknown) => { if (!controller.signal.aborted) failure(error); });
    return () => controller.abort();
  }, [seat.name, seat.session]);
  return <div className="network-managed" role="log" aria-label="Live agent transcript">{lines.map((line, index) => <div key={index} data-kind={line.kind}><span className="sec">{line.kind}</span><pre>{line.text}</pre></div>)}{!lines.length ? <p role="status">Waiting for session output…</p> : null}</div>;
}

export function SeatStart({ seat, change }: { seat: SeatView; change: (seat: SeatView) => void }) {
  const action = useSeatAction(seat, change);
  return <>{action.startAgain ? <Act symbol="start" name="Start again" word="Start again" className="small" disabled={action.busy} onClick={() => void action.run('restart')} /> : null}<Act symbol="start" name={action.failure ? 'Try again' : 'Start ' + seat.name} word={action.failure ? 'Try again' : 'Start'} className="small" disabled={action.busy} onClick={() => void action.run('start')} />
    {action.failure ? <ErrorWords problem={action.problem ?? action.failure} /> : null}
    {action.done ? <p role="status">{action.done}</p> : null}</>;
}

export function NetworkTreePane({ seat, name, reason, change }: { seat: SeatView; name: string; reason: string | null; change: (seat: SeatView) => void }) {
  const action = useSeatAction(seat, change);
  const [text, setText] = useState('');
  const [ended, setEnded] = useState<string | null>(null);
  const [failure, setFailure] = useState<unknown>(null);
  // Stop asks first, with the same question as on the agent's own page.
  const [asking, setAsking] = useState(false);
  const running = online(seat) && !action.stopEnded;
  const end = (value: SessionEnd) => setEnded('Session ' + seat.session + ' ended: ' + value.how + (value.status === null ? '' : ', status ' + value.status) + (value.signal ? ', ' + value.signal : '') + '.');
  return <section className="terminal network-terminal" aria-label={'Terminal of ' + name}>
    <header className="terminal-head"><h2 title={name}>{name}</h2><span role="status">{STATE_WORDS[seat.state] ?? 'State not known'}</span>
      {running && !ended ? <Act symbol="stop" name={RUN_WORDS.stop + ' ' + seat.name} word={RUN_WORDS.stop} className="small" disabled={action.busy} onClick={() => setAsking(true)} /> : null}
    </header>
    {asking && running && !ended ? <RunConfirm className="terminal-stop" small doing="stop" name={name} where={null} busy={action.busy} refused={null}
      waitingSince={action.waitingSince} endingNow={action.endingNow} endNow={() => { void action.endNow(); }}
      confirm={() => { void action.run('stop').then(() => setAsking(false)); }} cancel={() => setAsking(false)} /> : null}
    {reason ? <p className="network-reason" role="status" title={reason}>{problemWords(reason)}</p> : null}
    {running ? <form className="seat-send" aria-label={'Send to ' + seat.name} onSubmit={(event) => {
      event.preventDefault(); if (!text.trim() || ended) return;
      const submitted = text;
      void action.run('send', submitted).then((delivered) => { if (delivered) setText((current) => current === submitted ? '' : current); });
    }}><input aria-label={'Message for ' + seat.name} placeholder={'Message for ' + name} value={text} onChange={(event) => setText(event.target.value)} />
      <Act symbol="send" name={'Send to ' + seat.name} word="Send" type="submit" className="small" disabled={action.busy || Boolean(ended) || !text.trim()} /></form> : null}
    {action.done ? <p role="status">{action.done}</p> : null}
    {action.failure ? <ErrorWords problem={action.problem ?? action.failure} /> : null}
    {ended ? <p role="status">{ended}</p> : null}
    {failure ? <ErrorWords problem={failure} /> : null}
    {running && seat.session ? seat.transport === 'pty' ? <GpuTerminal key={seat.session} session={seat.session} onEnd={end} onFailure={(error) => setFailure(error)} />
      : seat.transport === 'managed' || seat.transport === 'acp' ? <ManagedTerminal key={seat.session} seat={seat} end={() => setEnded('Session ' + seat.session + ' ended.')} failure={setFailure} />
      : <p className="why-not" role="alert">This seat is running, but Lys does not know how to show its terminal. Stop it and start it again; if that does not help, ask the administrator to check its runner.</p>
      : !running && (seat.state === 'offline' || seat.state === 'not-seen' || seat.state === 'unknown') ? <div className="network-start">
        <p className="note">{seat.state === 'unknown' ? 'Lys cannot tell whether ' + name + ' is running. Start it to watch its terminal here; if it is already running, Lys says so.' : name + ' is not running. Start it to watch its terminal here.'}</p>
        <div className="chain"><SeatStart seat={seat} change={change} /></div>
      </div>
      : !seat.session && running ? <p className="why-not" role="alert">This seat reads as running, but Lys has no session for it to show. Stop it and start it again.</p> : null}
  </section>;
}
