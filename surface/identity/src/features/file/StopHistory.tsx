/** Saved emergency-stop outcomes are history, never a claim that a runtime has stopped. */
import { Refused, request, useLoad } from '../../api';
import { DirectoryGate as Gate } from '../people/Words';
import { clock } from './time';
import { ErrorWords, IdentityName } from '../people/Words';

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

/** How many were recorded; the identifiers themselves are kept under the stop's technical details, for whoever checks the record. */
function Counted({ values, one, many, empty }: { values: string[]; one: string; many: string; empty: string }) {
  return <span>{values.length ? values.length + ' ' + (values.length === 1 ? one : many) : empty}</span>;
}

/** The stop's request and every identifier it names, folded: a person reads the counts above; someone checking the record opens this. */
function Technical({ stop }: { stop: StopRecord }) {
  const named: [string, string[]][] = [['Certificates', stop.certificates_withdrawn], ['Credentials', stop.credentials_ended ?? []], ['Sessions', stop.sessions_asked]];
  return <details className="note"><summary>Technical details</summary>
    <p>Request <span className="mono">{stop.operation}</span></p>
    {named.filter(([, values]) => values.length).map(([what, values]) => <p key={what}>{what}: <span className="mono">{values.join(', ')}</span></p>)}
  </details>;
}

export function StopHistory({ id }: { id: string }) {
  const load = useLoad(async () => readStops(await request<unknown>('/agents/' + encodeURIComponent(id) + '/stops'), id), 'stop-history:' + id);
  return <section aria-label="Emergency-stop history" className="stop-history">
    <div className="head"><div><h2>Emergency-stop history</h2><p>Saved outcomes of earlier stops. These do not describe the agent’s current state.</p></div></div>
    <Gate load={load} title="Emergency-stop history" ok={(stops) => stops.length ? stops.map((stop) => <article className="card" key={stop.operation}>
      <h3>{stop.done ? 'Stop recorded' : 'Stop requested — outcome not confirmed'}</h3><p>{stop.reason}</p>
      <dl className="facts">
        <dt>Recorded at</dt><dd>{clock(stop.at)}</dd>
        <dt>Requested by</dt><dd><IdentityName id={stop.by} /></dd>
        {stop.done ? <>
        <dt>Authority after this stop</dt><dd>Suspended</dd>
        <dt>Certificates withdrawn</dt><dd><Counted values={stop.certificates_withdrawn} one="certificate" many="certificates" empty="None recorded for this stop." /></dd>
        <dt>Credentials ended</dt><dd>{stop.credentials_ended === null ? <div role="status"><p className="why-not">Not confirmed by the credential service.</p>{stop.credentials_refused ? <ErrorWords problem={stop.credentials_refused} /> : null}</div> : <Counted values={stop.credentials_ended} one="credential" many="credentials" empty="None recorded for this stop." />}</dd>
        <dt>Sessions asked to end</dt><dd><Counted values={stop.sessions_asked} one="session" many="sessions" empty="None recorded for this stop." /></dd>
        </> : null}
      </dl>
      {!stop.done ? <p role="status">The request was saved, but its outcome has not been recorded. This does not confirm suspension, certificate withdrawal, credential expiry or a session ending. Keep the original request when checking its outcome.</p> : null}
      <p className="note">An end request does not confirm a session stopped. <a href={'#/file/' + encodeURIComponent(id) + '/sessions'}>Check runtime reports</a>.</p>
      <Technical stop={stop} />
    </article>) : <div className="card"><p>No emergency-stop requests have been recorded for this agent.</p></div>} />
  </section>;
}
