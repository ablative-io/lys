/**
 * The proxy, read: every agent, the model calls its runs made, and one call whole, its request and its response as the
 * provider's own JSON, set out to be read. Three columns, each scrolling in itself; the address holds what is chosen,
 * so a call can be linked to.
 */
import { useEffect, useState } from 'react';
import { useSearchParams } from 'react-router';
import { Refused, useLive, useLoad } from '../../api';
import type { Load } from '../../api';
import type { Board } from '../dashboard/board';
import { refusedPart } from '../dashboard/contract';
import { clockOf } from '../file/time';
import { readCall, readCalls } from './contract';
import type { CallRow, CallView, CallsView, HeadSide } from './contract';
import './proxy.css';

const count = (value: number | undefined): string => value === undefined ? '' : value.toLocaleString('en-AU');
const took = (ms: number | null): string => ms === null ? '' : ms < 1000 ? ms + ' ms' : (ms / 1000).toLocaleString('en-AU', { maximumFractionDigits: 1 }) + ' s';
const unread = (refused: Refused, what: string) => <p className="why-not" role="alert">{what} could not be read. <small className="refusal-name" title={refused.refusal.reason}>{refused.refusal.refusal}</small></p>;

/** The address of the proxy view with these chosen. */
export const proxyHref = (chosen: { agent?: string; call?: string; shows?: string }): string =>
  '#/canvas?' + new URLSearchParams({ view: 'proxy', ...Object.fromEntries(Object.entries(chosen).filter(([, value]) => value)) }).toString();

function Agents({ board, chosen }: { board: Load<Board>; chosen: string | null }) {
  if (board.status === 'loading') return <p role="status">Reading the agents…</p>;
  if (board.status === 'refused') return unread(board.refused, 'The agents');
  return <table className="usage-table proxy-agents"><thead><tr><th>Agent</th><th>Now</th></tr></thead><tbody>
    {board.data.rows.map((row) => <tr key={row.agent.id} data-agent={row.agent.id} aria-current={chosen === row.agent.id ? 'true' : undefined}>
      <td><a href={proxyHref({ agent: row.agent.id })}>{row.agent.display_name}</a></td>
      <td className="sec">{refusedPart(row.sessions) ? <small className="refusal-name" title={row.sessions.reason}>{row.sessions.refusal}</small> : row.sessions.length ? 'Running' : 'Not running'}</td></tr>)}
    {board.data.rows.length ? null : <tr className="empty"><td className="dim" colSpan={2}>There is no agent you may see.</td></tr>}
  </tbody></table>;
}

function Calls({ agent, chosen }: { agent: string; chosen: string | null }) {
  const first = useLive(() => readCalls(agent), 'proxy-calls:' + agent);
  // Earlier calls are asked for a page at a time and kept under what the first read answered.
  const [earlier, setEarlier] = useState<CallsView[]>([]);
  const [refused, setRefused] = useState<Refused | null>(null);
  useEffect(() => { setEarlier([]); setRefused(null); }, [agent]);
  if (first.status === 'loading') return <p role="status">Reading its calls…</p>;
  if (first.status === 'refused') return unread(first.refused, 'Its calls');
  const pages = [first.data, ...earlier];
  const rows: CallRow[] = pages.flatMap((page) => page.calls);
  const after = pages[pages.length - 1].after;
  return <table className="usage-table proxy-calls"><thead><tr><th>Started</th><th>Model</th><th>Ended</th><th className="num">In</th><th className="num">Out</th><th className="num">From cache</th><th className="num">Took</th></tr></thead><tbody>
    {rows.map((row) => <tr key={row.call_id} data-call={row.call_id} aria-current={chosen === row.call_id ? 'true' : undefined}>
      <td><a href={proxyHref({ agent, call: row.call_id })}>{clockOf(row.started_at)}</a></td>
      <td>{row.model ?? <span className="dim">Not readable</span>}</td>
      <td className={row.status === 'complete' ? 'sec' : 'why-not'} title={row.unrecorded_reason ?? undefined}>{row.status}{row.http_status !== null && row.http_status !== 200 ? ' (' + row.http_status + ')' : ''}</td>
      <td className="num">{count(row.usage?.input)}</td><td className="num">{count(row.usage?.output)}</td><td className="num">{count(row.usage?.cache_read)}</td>
      <td className="num">{took(row.duration_ms)}</td></tr>)}
    {rows.length ? null : <tr className="empty"><td className="dim" colSpan={7}>No call of this agent has been recorded.</td></tr>}
  </tbody>
  {after || refused ? <tfoot><tr><td colSpan={7}>
    {after ? <button type="button" className="btn" data-act="earlier-calls" onClick={() => { readCalls(agent, after).then((page) => setEarlier((now) => [...now, page]), (problem: unknown) => setRefused(problem instanceof Refused ? problem : new Refused(0, { refusal: 'Unanswered', reason: String(problem) }))); }}>Earlier calls</button> : null}
    {refused ? unread(refused, 'The earlier calls') : null}
  </td></tr></tfoot> : null}</table>;
}

const SHOWS = { request: 'Request', response: 'Response', headers: 'Headers' } as const;
type Shows = keyof typeof SHOWS;

function Side({ name, side }: { name: string; side: HeadSide | null }) {
  if (!side) return <tr className="empty"><td className="dim" colSpan={2}>No {name} headers are on the record.</td></tr>;
  return <>{side.names.map((header, index) => <tr key={name + index} data-header={header}><td>{header}</td>
    <td className="sec">{side.values[header] ? side.values[header].join(', ') : <span className="dim">Value not kept</span>}</td></tr>)}</>;
}

/** A body set out to be read: the provider's JSON, two spaces to a level, nothing left out. */
function Body({ value, why, what }: { value: unknown; why: string | null; what: string }) {
  const [copied, setCopied] = useState(false);
  if (value === null || value === undefined) return <p className="why-not" role="status">This call has no {what} that can be read as JSON.{why ? ' ' + why : ''}</p>;
  const text = JSON.stringify(value, null, 2);
  return <>
    <button type="button" className="btn proxy-copy" data-act="copy-json" onClick={() => { void navigator.clipboard?.writeText(text).then(() => setCopied(true)); }}>{copied ? 'Copied' : 'Copy'}</button>
    <pre className="proxy-json" data-json={what} tabIndex={0}>{text}</pre>
  </>;
}

function Call({ agent, call, shows }: { agent: string; call: string; shows: Shows }) {
  const load: Load<CallView> = useLoad(() => readCall(agent, call), 'proxy-call:' + agent + ':' + call);
  if (load.status === 'loading') return <p role="status">Reading the call…</p>;
  if (load.status === 'refused') return unread(load.refused, 'The call');
  const { call: row, request, response, request_head, response_head, request_unreadable, response_unreadable } = load.data;
  return <>
    <p className="proxy-call-line">{row.model ?? 'Model not readable'} · {row.status}{row.http_status !== null ? ' · HTTP ' + row.http_status : ''}{row.duration_ms !== null ? ' · ' + took(row.duration_ms) : ''}{row.stream ? ' · streamed' : ''}
      {row.request_id ? <> · <small className="refusal-name">{row.request_id}</small></> : null}</p>
    {row.unrecorded_reason ? <p className="why-not" role="status">{row.unrecorded_reason}</p> : null}
    <div className="canvas-widget-variants proxy-shows" role="toolbar" aria-label="What of the call is shown">
      {(Object.keys(SHOWS) as Shows[]).map((key) => <a key={key} href={proxyHref({ agent, call, shows: key === 'request' ? undefined : key })} data-shows={key} aria-current={shows === key ? 'true' : undefined}>{SHOWS[key]}</a>)}
    </div>
    {shows === 'request' ? <Body value={request} why={request_unreadable} what="request" /> : null}
    {shows === 'response' ? <Body value={response} why={response_unreadable} what="response" /> : null}
    {shows === 'headers' ? <table className="usage-table proxy-headers"><tbody>
      <tr><th colSpan={2}>Request</th></tr><Side name="request" side={request_head} />
      <tr><th colSpan={2}>Response</th></tr><Side name="response" side={response_head} />
    </tbody></table> : null}
  </>;
}

/** The proxy view: the agents, the chosen agent's calls, the chosen call. */
export function ProxyView({ board }: { board: Load<Board> }) {
  const [search] = useSearchParams();
  const [agent, call] = [search.get('agent'), search.get('call')];
  const shows: Shows = search.get('shows') === 'response' ? 'response' : search.get('shows') === 'headers' ? 'headers' : 'request';
  return <div className="proxy-view">
    <section aria-label="Agents"><Agents board={board} chosen={agent} /></section>
    <section aria-label="Calls">{agent ? <Calls agent={agent} chosen={call} /> : <p className="dim">Choose an agent to see the model calls its runs made.</p>}</section>
    <section aria-label="The call">{agent && call ? <Call agent={agent} call={call} shows={shows} /> : <p className="dim">Choose a call to see its request and its response.</p>}</section>
  </div>;
}
