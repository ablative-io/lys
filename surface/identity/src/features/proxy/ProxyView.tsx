/**
 * The proxy, read: every agent, the model calls its runs made, and one call whole: first read as what went in and what
 * came out, then its request and its response as the provider's own JSON, in colour. Three columns, each scrolling in itself; the address holds what is chosen,
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
import { Coloured } from './json-colour';
import { ProxyRead } from './ProxyRead';
import type { CallRow, CallView, CallsView, HeadSide } from './contract';

import './proxy.css';
import { Act } from '../../shell/Act';
import { ErrorWords } from '../people/Words';

const HTTP_OK = 200;
/** How a call ended, in words for the list, and what the word means. */
const ENDED: Partial<Record<string, [string, string]>> = {
  complete: ['Complete', 'The provider answered in full and reported what the call used.'],
  unrecorded: ['No complete response', 'What the provider answered was not a complete response, so no usage was reported. Open the call to see what it answered.'],
  lost: ['Lost in flight', 'The proxy stopped before the response ended. What the call spent is not known.'],
};
/** How a call ended, in words; a word this page does not know is said as the record has it, with a capital. */
const endedWords = (status: string): string => ENDED[status]?.[0] ?? status.charAt(0).toUpperCase() + status.slice(1).replace(/_/g, ' ');

const count = (value: number | null): string => value === null ? '' : value.toLocaleString('en-AU');
const took = (ms: number | null | undefined): string => ms === null || ms === undefined ? '' : ms < 1000 ? ms + ' ms' : (ms / 1000).toLocaleString('en-AU', { maximumFractionDigits: 1 }) + ' s';
const unread = (refused: Refused, what: string) => <div className="proxy-unread"><p className="why-not">{what} could not be read.</p><ErrorWords problem={refused} /></div>;

/** The address of the proxy view with these chosen. */
export const proxyHref = (chosen: { agent?: string; call?: string; shows?: string }): string =>
  '#/canvas?' + new URLSearchParams({ view: 'proxy', ...Object.fromEntries(Object.entries(chosen).filter(([, value]) => value)) }).toString();

/** The agents as a filterable list: a search over name and id, running first, retired ones behind a switch. */
function Agents({ board, chosen }: { board: Load<Board>; chosen: string | null }) {
  const [search, setSearch] = useState('');
  const [retired, setRetired] = useState(false);
  if (board.status === 'loading') return <p role="status">Reading the agents…</p>;
  if (board.status === 'refused') return unread(board.refused, 'The agents');
  const every = [...new Map(board.data.rows.map(row => [row.agent.id, row])).values()];
  const running = (row: typeof every[number]): boolean => !refusedPart(row.sessions) && row.sessions.length > 0;
  const needle = search.trim().toLowerCase();
  const shown = every
    .filter((row) => retired || row.agent.state !== 'retired' || row.agent.id === chosen)
    .filter((row) => !needle || row.agent.display_name.toLowerCase().includes(needle) || row.agent.id.toLowerCase().includes(needle))
    .sort((a, b) => Number(running(b)) - Number(running(a)) || a.agent.display_name.localeCompare(b.agent.display_name, 'en-AU'));
  const hidden = every.length - shown.length;
  return <>
    <div className="proxy-agents-find">
      <input type="search" value={search} placeholder="Find an agent" aria-label="Find an agent by name" onChange={(event) => setSearch(event.target.value)} />
      <label className="dim"><input type="checkbox" checked={retired} onChange={(event) => setRetired(event.target.checked)} /> Show retired</label>
    </div>
    <table className="usage-table proxy-agents"><thead><tr><th>Agent</th><th>Now</th></tr></thead><tbody>
      {shown.map((row) => <tr key={row.agent.id} data-agent={row.agent.id} data-running={running(row)} aria-current={chosen === row.agent.id ? 'true' : undefined}>
        <td><a title={row.agent.id} href={proxyHref({ agent: row.agent.id })}>{row.agent.display_name}</a>{row.agent.state === 'retired' ? <small className="dim"> retired</small> : null}</td>
        <td className="sec">{refusedPart(row.sessions) ? <span className="dim" title={row.sessions.reason}>Cannot tell</span> : running(row) ? 'Running' : 'Not running'}</td></tr>)}
      {shown.length ? null : <tr className="empty"><td className="dim" colSpan={2}>{every.length ? 'No agent matches.' : 'There is no agent you may see.'}</td></tr>}
    </tbody>
    {hidden ? <tfoot><tr><td className="dim" colSpan={2}>{hidden.toLocaleString('en-AU')} {hidden === 1 ? 'agent' : 'agents'} not shown{needle ? ' by the search' : ' because retired'}.</td></tr></tfoot> : null}</table>
  </>;
}

function Calls({ agent, chosen }: { agent: string; chosen: string | null }) {
  const first = useLive(() => readCalls(agent), 'proxy-calls:' + agent);
  // Earlier calls are asked for a page at a time and kept under what the first read answered.
  const [earlier, setEarlier] = useState<CallsView[]>([]);
  const [refused, setRefused] = useState<Refused | null>(null);
  const [reading, setReading] = useState(false);
  useEffect(() => { setEarlier([]); setRefused(null); setReading(false); }, [agent]);
  if (first.status === 'loading') return <p role="status">Reading its calls…</p>;
  if (first.status === 'refused') return unread(first.refused, 'Its calls');
  const pages = [first.data, ...earlier];
  const rows: CallRow[] = pages.flatMap((page) => page.calls);
  const after = pages[pages.length - 1].next;
  return <table className="usage-table proxy-calls"><thead><tr><th>At</th><th>Model</th><th>Ended</th><th className="num">In</th><th className="num">Out</th><th className="num" title="Tokens read from the provider's cache">Cached</th><th className="num">Took</th></tr></thead><tbody>
    {rows.map((row) => <tr key={row.call_id} data-call={row.call_id} aria-current={chosen === row.call_id ? 'true' : undefined}>
      <td><a href={proxyHref({ agent, call: row.call_id })}>{clockOf(new Date(row.at_ms).toISOString())}</a></td>
      <td>{row.model ?? <span className="dim">Not readable</span>}</td>
      <td className={row.status === 'complete' ? 'sec' : 'why-not'} title={row.status ? ENDED[row.status]?.[1] : undefined}>{row.status ? endedWords(row.status) : <span className="dim">Not said</span>}</td>
      <td className="num">{count(row.input_tokens)}</td><td className="num">{count(row.output_tokens)}</td><td className="num">{count(row.cache_read_tokens)}</td>
      <td className="num">{took(row.duration_ms)}</td></tr>)}
    {rows.length ? null : <tr className="empty"><td className="dim" colSpan={7}>No call of this agent has been recorded.</td></tr>}
  </tbody>
  {after || refused ? <tfoot><tr><td colSpan={7}>
    {after ? <Act symbol="add" name={reading ? 'Reading earlier calls…' : 'Earlier calls'} word={reading ? 'Reading…' : 'Earlier'} data-act="earlier-calls" disabled={reading} onClick={() => {
      if (reading) return;
      setReading(true); setRefused(null);
      readCalls(agent, after).then((page) => setEarlier((now) => [...now, page]), (problem: unknown) => setRefused(problem instanceof Refused ? problem : new Refused(0, { refusal: 'Unanswered', reason: String(problem) }))).finally(() => setReading(false));
    }} /> : null}
    {refused ? unread(refused, 'The earlier calls') : null}
  </td></tr></tfoot> : null}</table>;
}

const SHOWS = { read: 'Read', request: 'Request', response: 'Response', headers: 'Headers' } as const;
type Shows = keyof typeof SHOWS;

function Side({ name, side }: { name: string; side: HeadSide | undefined }) {
  if (!side) return <tr className="empty"><td className="dim" colSpan={2}>No {name} headers are on the record.</td></tr>;
  return <>{side.names.map((header, index) => <tr key={name + index} data-header={header}><td>{header}</td>
    <td className="sec">{side.values[header] ? side.values[header].join(', ') : <span className="dim">Value not kept</span>}</td></tr>)}</>;
}

/** A body set out to be read: the provider's JSON, two spaces to a level, nothing left out. */
function Body({ value, why, what }: { value: unknown; why: string | null; what: string }) {
  const [copied, setCopied] = useState(false);
  const [uncopied, setUncopied] = useState('');
  if (value === null || value === undefined) return <p className="why-not" role="status">This call has no {what} that can be read as JSON.{why ? ' ' + why : ''}</p>;
  const text = JSON.stringify(value, null, 2);
  return <>
    <Act symbol={copied ? 'approve' : 'copy'} name={copied ? 'Copied' : 'Copy the ' + what} word={copied ? 'Copied' : 'Copy'} className="proxy-copy" data-act="copy-json" onClick={() => {
      setUncopied('');
      if (!navigator.clipboard) { setUncopied('This browser does not let the page copy. Select the text and copy it yourself.'); return; }
      navigator.clipboard.writeText(text).then(() => setCopied(true), () => setUncopied('The browser did not allow copying. Select the text and copy it yourself.'));
    }} />
    {uncopied ? <p className="why-not proxy-uncopied" role="alert">{uncopied}</p> : null}
    <pre className="proxy-json" data-json={what} tabIndex={0}><Coloured text={text} /></pre>
  </>;
}

function Call({ agent, call, shows }: { agent: string; call: string; shows: Shows }) {
  const load: Load<CallView> = useLoad(() => readCall(agent, call), 'proxy-call:' + agent + ':' + call);
  if (load.status === 'loading') return <p role="status">Reading the call…</p>;
  if (load.status === 'refused') return unread(load.refused, 'The call');
  const { call: row, request, response, request_unreadable, response_unreadable } = load.data;
  const http = row.head?.status;
  return <>
    <p className="proxy-call-line" data-http={http !== undefined && http !== null && http !== HTTP_OK ? 'refused' : undefined}>{row.model ?? 'Model not readable'} · {row.status ? endedWords(row.status) : 'Ending not said'}{http !== undefined && http !== null ? ' · HTTP ' + http : ''}{row.duration_ms !== undefined ? ' · ' + took(row.duration_ms) : ''}{row.stream ? ' · streamed' : ''}
      {row.request_id ? <> · <span title="The provider's own id for this request, to quote to the provider">Request <small className="mono">{row.request_id}</small></span></> : null}</p>
    {row.unrecorded_reason ? <p className="why-not" role="status">{row.unrecorded_reason}</p> : null}
    <div className="canvas-widget-variants proxy-shows" role="toolbar" aria-label="What of the call is shown">
      {(Object.keys(SHOWS) as Shows[]).map((key) => <a key={key} href={proxyHref({ agent, call, shows: key === 'read' ? undefined : key })} data-shows={key} aria-current={shows === key ? 'true' : undefined}>{SHOWS[key]}</a>)}
    </div>
    {shows === 'read' ? <ProxyRead request={request} response={response} unreadable={{ request: request_unreadable, response: response_unreadable }} /> : null}
    {shows === 'request' ? <Body value={request} why={request_unreadable} what="request" /> : null}
    {shows === 'response' ? <Body value={response} why={response_unreadable} what="response" /> : null}
    {shows === 'headers' ? <table className="usage-table proxy-headers"><tbody>
      <tr><th colSpan={2}>Request</th></tr><Side name="request" side={row.head?.request} />
      <tr><th colSpan={2}>Response</th></tr><Side name="response" side={row.head?.response} />
    </tbody></table> : null}
  </>;
}

/** The proxy view: the agents, the chosen agent's calls, the chosen call. */
export function ProxyView({ board }: { board: Load<Board> }) {
  const [search] = useSearchParams();
  const [agent, call] = [search.get('agent'), search.get('call')];
  const asked = search.get('shows');
  const shows: Shows = asked !== null && asked in SHOWS ? asked as Shows : 'read';
  return <div className="proxy-view">
    <section aria-label="Agents"><Agents board={board} chosen={agent} /></section>
    <section aria-label="Calls">{agent ? <Calls agent={agent} chosen={call} /> : <p className="dim">Choose an agent on the left to see the model calls it made.</p>}</section>
    <section aria-label="The call">{agent && call ? <Call agent={agent} call={call} shows={shows} /> : <p className="dim">{agent ? 'Choose a call to see what was sent and what came back.' : 'Choose an agent, then one of its calls.'}</p>}</section>
  </div>;
}
