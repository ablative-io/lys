/**
 * The notice of a launch record's state (DIRECTORY-029 R13). Unconfirmed
 * until the agent's own signed report names the record: the request stands,
 * and asking elsewhere could start it twice, so the notice offers what
 * answers it, waiting for the report or withdrawing the request. A
 * withdrawal says only that the request no longer stands, and a report
 * arriving after it shows running with the withdrawal beside it.
 */
import { useState } from 'react';
import { API } from '../../api';
import type { LaunchStateView } from '../../generated';

/** The start route's answer: its JSON as the library wrote it, or its refusal in words. */
export type Asked<T> =
  | { ok: true; status: number; body: T }
  | { ok: false; status: number; words: string; body: unknown };

/** The words of a refusal the start route answered, whichever shape it has. */
function wordsOf(body: unknown): string {
  const answer = (body ?? {}) as { refused?: { words: string }[]; words?: string; refusal?: string; reason?: string };
  if (Array.isArray(answer.refused)) return answer.refused.map((refusal) => refusal.words).join(' ');
  if (typeof answer.words === 'string') return answer.words;
  if (typeof answer.refusal === 'string') return answer.refusal + ': ' + (answer.reason ?? '');
  return 'the identity service refused without naming why';
}

/** Ask the start route and read its JSON, given or refused, exactly as it came. */
export async function ask<T>(method: 'GET' | 'POST', path: string, body: unknown = {}): Promise<Asked<T>> {
  const init: RequestInit = method === 'POST'
    ? { method, credentials: 'same-origin', headers: { accept: 'application/json', 'content-type': 'application/json' }, body: JSON.stringify(body) }
    : { method, credentials: 'same-origin', headers: { accept: 'application/json' } };
  let response: Response;
  try {
    response = await fetch(API + path, init);
  } catch (error) {
    return { ok: false, status: 0, words: 'the identity service could not be reached: ' + String(error), body: null };
  }
  let parsed: unknown;
  try {
    parsed = await response.json();
  } catch {
    return { ok: false, status: response.status, words: `the identity service answered ${response.status}, but its answer could not be read`, body: null };
  }
  return response.ok ? { ok: true, status: response.status, body: parsed as T } : { ok: false, status: response.status, words: wordsOf(parsed), body: parsed };
}

const when = (at: number): string => new Date(at * 1000).toISOString();

export function StartNotice({ initial }: { initial: LaunchStateView }) {
  const [state, setState] = useState<LaunchStateView>(initial);
  const [problem, setProblem] = useState('');
  const path = '/launch-records/' + encodeURIComponent(state.launch_record);
  const act = async (method: 'GET' | 'POST', to: string) => {
    const answer = await ask<LaunchStateView>(method, to);
    if (answer.ok) {
      setState(answer.body);
      setProblem('');
    } else {
      setProblem(answer.words);
    }
  };
  const withdrawal = state.withdrawal ? (
    <span className="dim" data-withdrawal>
      {' '}withdrawn by {state.withdrawal.by} at {when(state.withdrawal.at)}
    </span>
  ) : null;
  return (
    <section className="card" data-launch-state={state.state} aria-live="polite">
      <h3>Launch record {state.launch_record}</h3>
      {state.state === 'unconfirmed' ? (
        <div role="status">
          <b>Unconfirmed.</b> The request stands. No signed report names this launch record yet. Asking another
          machine could start it twice: wait for its report, or withdraw it first.
        </div>
      ) : state.state === 'running' ? (
        <div role="status">
          <b>running</b>, on its signed report of session {state.session}.{withdrawal}
        </div>
      ) : (
        <div role="status">
          <b>withdrawn</b>: the request no longer stands.{withdrawal}
        </div>
      )}
      <div style={{ display: 'flex', gap: 8, marginTop: 8 }}>
        <button className="btn" data-act="read-state" onClick={() => void act('GET', path + '/state')}>
          Wait for its report: read its state
        </button>
        {state.state === 'unconfirmed' ? (
          <button className="btn danger" data-act="withdraw" onClick={() => void act('POST', path + '/withdraw')}>
            Withdraw the request
          </button>
        ) : null}
      </div>
      {problem ? <p className="why-not">{problem}</p> : null}
    </section>
  );
}
