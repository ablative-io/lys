/** One session's terminal, read live through its runner: each read follows on from the last cursor and is answered when output arrives or the session ends, and leaving the screen closes the read. Typed lines, keys and Stop each go through Lys under the caller's grant; a refusal is shown by name. */
import { useEffect, useRef, useState } from 'react';
import type { FormEvent } from 'react';
import { API, Refused, api, request, useLoad } from '../../api';
import { clock } from '../file/time';

export interface SessionEnd { how: 'exited' | 'ended_by_runner_restart' | 'accounts_exhausted'; at: number; status: number | null; signal: string | null }
interface Output { session: string; from: number; cursor: number; oldest: number; text: string; ended: SessionEnd | null }
interface ReadAnswer { answer?: { kind?: string; output?: Output } }
interface EndAnswer { answer?: { kind?: string; ended?: SessionEnd } }

/** The keys offered in one press each, by the name the runner takes. */
export const KEYS: readonly (readonly [string, string])[] = [
  ['enter', 'Enter'], ['tab', 'Tab'], ['escape', 'Esc'], ['up', '↑'], ['down', '↓'], ['ctrl_c', 'Ctrl-C'], ['ctrl_d', 'Ctrl-D'],
];

/** Terminal control sequences are not drawn; the text is shown as the runner kept it without them. */
const CONTROL = new RegExp(String.raw`\x1b\[[0-9;?]*[ -/]*[@-~]|\x1b\][^\x07\x1b]*(?:\x07|\x1b\\)|\x1b[()][A-Za-z0-9]|\x1b[=>]|\r`, 'g');

export const clean = (text: string): string => text.replace(CONTROL, '');

const asRefused = (error: unknown): Refused =>
  error instanceof Refused ? error : new Refused(0, { refusal: 'Unanswered', reason: String(error) });

/** A read that follows, closed when `signal` aborts so the runner stops waiting on it. */
async function follow(path: string, body: unknown, signal: AbortSignal): Promise<ReadAnswer> {
  let response: Response;
  try {
    response = await fetch(API + path, {
      method: 'POST', credentials: 'same-origin', signal,
      headers: { accept: 'application/json', 'content-type': 'application/json' }, body: JSON.stringify(body),
    });
  } catch (error) {
    throw new Refused(0, { refusal: 'ServiceUnreachable', reason: `the identity service could not be reached: ${String(error)}` });
  }
  let answer: unknown;
  try {
    answer = await response.json();
  } catch {
    throw new Refused(response.status, { refusal: 'UnreadableResponse', reason: `the identity service answered ${response.status}, but its result could not be read` });
  }
  if (!response.ok) {
    const named = answer as { refusal?: unknown; reason?: unknown };
    if (typeof named.refusal === 'string' && typeof named.reason === 'string') throw new Refused(response.status, { refusal: named.refusal, reason: named.reason });
    throw new Refused(response.status, { refusal: 'Unanswered', reason: `the service answered ${response.status} without naming a refusal` });
  }
  return answer as ReadAnswer;
}

function endWords(ended: SessionEnd): string {
  const how = ended.how === 'exited' ? 'The process exited' : ended.how === 'accounts_exhausted' ? 'Stopped at a usage limit on its last account' : 'Ended when its runner restarted';
  const status = ended.status !== null ? `, status ${ended.status}` : ended.signal ? `, ${ended.signal}` : ', no exit status';
  return `${how}${status}, ${clock(Math.floor(ended.at / 1000))}.`;
}

export function Terminal({ session, agent }: { session: string; agent: string | null }) {
  const base = '/runtime/sessions/' + encodeURIComponent(session);
  const [text, setText] = useState('');
  const [ended, setEnded] = useState<SessionEnd | null>(null);
  const [refused, setRefused] = useState<Refused | null>(null);
  const [confirming, setConfirming] = useState(false);
  const [busy, setBusy] = useState(false);
  const screen = useRef<HTMLPreElement>(null);
  const named = useLoad(async () => (agent ? (await api.agent(agent)).display_name : session), 'terminal-agent:' + (agent ?? session));
  const name = named.status === 'ok' ? named.data : agent ?? session;

  useEffect(() => {
    const controller = new AbortController();
    let cursor: number | null = null;
    const run = async () => {
      for (;;) {
        let answer: ReadAnswer;
        try {
          answer = await follow(base + '/read', { cursor, follow: true }, controller.signal);
        } catch (error) {
          if (!controller.signal.aborted) setRefused(asRefused(error));
          return;
        }
        if (controller.signal.aborted) return;
        const output = answer.answer?.output;
        if (!output || typeof output.cursor !== 'number' || typeof output.text !== 'string') {
          setRefused(new Refused(0, { refusal: 'UnreadableResponse', reason: 'the read did not answer the session’s output' }));
          return;
        }
        cursor = output.cursor;
        const shown = clean(output.text);
        if (shown) setText((before) => before + shown);
        if (output.ended) {
          setEnded(output.ended);
          return;
        }
      }
    };
    void run();
    return () => controller.abort();
  }, [base]);

  useEffect(() => {
    const pre = screen.current;
    if (pre) pre.scrollTop = pre.scrollHeight;
  }, [text]);

  const send = async (path: string, body: unknown) => {
    setBusy(true);
    setRefused(null);
    try {
      return await request<EndAnswer>(base + path, body);
    } catch (error) {
      setRefused(asRefused(error));
      return null;
    } finally {
      setBusy(false);
    }
  };

  const typed = async (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    const form = event.currentTarget;
    const line = String(new FormData(form).get('line') ?? '');
    if (await send('/input', { text: line, enter: true })) form.reset();
  };

  const stop = async () => {
    const answer = await send('/end', {});
    setConfirming(false);
    if (answer?.answer?.ended) setEnded(answer.answer.ended);
  };

  return <section className="terminal" aria-label={'Terminal of ' + name}>
    <div className="terminal-head">
      <div><h2>{name}</h2><p className="mono">{session}</p></div>
      <p className="terminal-state" role="status" data-ended={ended ? 'true' : 'false'}>{ended ? endWords(ended) : 'Running, read live from its runner.'}</p>
      {!ended ? <button type="button" className="btn" data-act="stop" disabled={busy} onClick={() => setConfirming(true)}>Stop</button> : null}
    </div>
    {confirming ? <div className="terminal-stop" role="alertdialog" aria-label={'Stop ' + name}>
      <p>Stop {name}? This ends its session on the machine it runs on. What it has not saved is lost.</p>
      <div className="actions">
        <button type="button" className="btn danger" data-act="confirm-stop" disabled={busy} onClick={() => void stop()}>Stop {name}</button>
        <button type="button" className="btn" data-act="keep-running" onClick={() => setConfirming(false)}>Keep it running</button>
      </div>
    </div> : null}
    <pre className="terminal-screen" ref={screen} aria-live="polite">{text}</pre>
    {!ended ? <>
      <form className="terminal-line" aria-label="Type to the session" onSubmit={(event) => void typed(event)}>
        <input name="line" aria-label="Line to type" autoComplete="off" spellCheck={false} disabled={busy} />
        <button type="submit" className="btn primary" disabled={busy}>Send</button>
      </form>
      <div className="terminal-keys" aria-label="Keys">
        {KEYS.map(([key, label]) => <button type="button" className="btn" key={key} data-key={key} disabled={busy} onClick={() => void send('/keys', { keys: [key] })}>{label}</button>)}
      </div>
    </> : null}
    {refused ? <p className="why-not" role="alert">{refused.refusal.refusal}: {refused.refusal.reason}</p> : null}
  </section>;
}
