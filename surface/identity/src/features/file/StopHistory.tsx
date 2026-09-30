/** Saved emergency-stop outcomes are history, never a claim that a runtime has stopped. */
import { Refused, request, useLoad } from '../../api';
import { DirectoryGate as Gate } from '../people/Words';

interface StopRecord {
  agent: string;
  operation: string;
  state: string;
  done: boolean;
  by: string;
  at: number;
  reason: string;
  certificates_withdrawn: string[];
  credentials_ended: string[] | null;
  credentials_refused: string | null;
  sessions_asked: string[];
}

const strings = (value: unknown): value is string[] => Array.isArray(value) && value.every((part) => typeof part === 'string');
function isStop(value: unknown, id: string): value is StopRecord {
  if (typeof value !== 'object' || value === null) return false;
  const row = value as Record<string, unknown>;
  const outcome = row.done === false
    ? row.state === 'asked' && strings(row.certificates_withdrawn) && row.certificates_withdrawn.length === 0
      && strings(row.sessions_asked) && row.sessions_asked.length === 0 && row.credentials_ended === null && row.credentials_refused === null
    : row.done === true && row.state === 'suspended'
      && ((strings(row.credentials_ended) && row.credentials_refused === null)
        || (row.credentials_ended === null && typeof row.credentials_refused === 'string' && row.credentials_refused.length > 0));
  return row.agent === id && outcome
    && typeof row.operation === 'string' && row.operation.length > 0
    && typeof row.by === 'string' && row.by.length > 0 && typeof row.reason === 'string'
    && typeof row.at === 'number' && Number.isSafeInteger(row.at) && row.at >= 0
    && Number.isFinite(new Date(row.at * 1000).getTime())
    && strings(row.certificates_withdrawn) && strings(row.sessions_asked);
}

function readStops(answer: unknown, id: string): StopRecord[] {
  if (typeof answer === 'object' && answer !== null && 'stops' in answer && Array.isArray(answer.stops)
    && answer.stops.every((row: unknown) => isStop(row, id))) {
    const rows: StopRecord[] = answer.stops;
    if (new Set(rows.map((row) => row.operation)).size === rows.length) return rows;
  }
  throw new Refused(200, { refusal: 'StopHistoryUnreadable', reason: 'The saved-stop answer did not name this agent and a valid, distinct outcome for each operation.' });
}

function Identifiers({ values, empty }: { values: string[]; empty: string }) {
  return values.length ? <ul>{values.map((value) => <li key={value} className="mono">{value}</li>)}</ul> : <span>{empty}</span>;
}

export function StopHistory({ id }: { id: string }) {
  const load = useLoad(async () => readStops(await request<unknown>('/agents/' + encodeURIComponent(id) + '/stops'), id), 'stop-history:' + id);
  return <section aria-label="Emergency-stop history">
    <div className="head"><div><h2>Emergency-stop history</h2><p>Saved outcomes of earlier stops. These do not describe the agent’s current state.</p></div></div>
    <Gate load={load} title="Emergency-stop history" ok={(stops) => stops.length ? stops.map((stop) => <article className="card" key={stop.operation}>
      <h3>{stop.done ? 'Stop recorded' : 'Stop requested — outcome not confirmed'}</h3><p>{stop.reason}</p>
      <dl className="facts">
        <dt>Recorded at</dt><dd>{new Date(stop.at * 1000).toLocaleString('en-AU', { timeZone: 'Australia/Melbourne', timeZoneName: 'short' })}</dd>
        <dt>Requested by</dt><dd><a href={'#/file/' + encodeURIComponent(stop.by)}>{stop.by}</a></dd>
        {stop.done ? <>
        <dt>Authority after this stop</dt><dd>Suspended</dd>
        <dt>Certificates withdrawn</dt><dd><Identifiers values={stop.certificates_withdrawn} empty="None recorded for this stop." /></dd>
        <dt>Credentials ended</dt><dd>{stop.credentials_ended === null ? <span role="status">Not confirmed: {stop.credentials_refused}</span> : <Identifiers values={stop.credentials_ended} empty="None recorded for this stop." />}</dd>
        <dt>Sessions asked to end</dt><dd><Identifiers values={stop.sessions_asked} empty="None recorded for this stop." /></dd>
        </> : null}
      </dl>
      {!stop.done ? <p role="status">The request was saved, but its outcome has not been recorded. This does not confirm suspension, certificate withdrawal, credential expiry or a session ending. Keep the original request when checking its outcome.</p> : null}
      <p className="note">An end request does not confirm a session stopped. <a href={'#/file/' + encodeURIComponent(id) + '/sessions'}>Check runtime reports</a>.</p>
      <details><summary>Operation details</summary><p className="mono">{stop.operation}</p></details>
    </article>) : <div className="card"><p>No emergency-stop requests have been recorded for this agent.</p></div>} />
  </section>;
}
