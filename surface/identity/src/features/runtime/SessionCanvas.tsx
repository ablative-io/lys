import { useRef, useState } from 'react';
import { useParams, useSearchParams } from 'react-router';
import { useLive, useLoad } from '../../api';
import type { Load } from '../../api';
import { refreshLive } from '../../live';
import { Gate } from '../signin/Gate';
import { Act } from '../../shell/Act';
import { RunningList } from './Sessions';
import { Seats } from '../sessions/Seats';
import { ProxyView, proxyHref } from '../proxy/ProxyView';
import { readBoard } from '../dashboard/board';
import type { Board } from '../dashboard/board';
import { readSessionGraph, withMessages } from './session-graph';
import type { SessionGraph } from './session-graph';
import { firstMessagePage } from './message-connections';
import type { MessageRead } from './message-connections';
import { MessageConnections } from './MessageConnections';
import { keepOperations, readKeeping, said } from './canvas-kept';
import type { Keeping, OperationsChoice } from './canvas-kept';
import { Canvas } from './CanvasSurface';
import { NetworkTree } from './NetworkTree';
export { lineBetween, placed } from './canvas-place';
export type { Box } from './canvas-marks';
export type { View } from './canvas-view';

/** A session whose runner did not answer, said with the agent's name and the computer's, never their identifiers; the runner's refusal follows, small. */
function Unanswered({ graph, entry }: { graph: SessionGraph; entry: SessionGraph['unanswered'][number] }) {
  const session = graph.nodes.find((node) => node.session?.session === entry.session)?.session;
  const agent = session?.agent ? graph.nodes.find((node) => node.session?.session === entry.session)?.title : undefined;
  const who = agent && agent !== session?.agent ? agent : session?.agent ? 'an agent outside your view' : 'a session with no agent';
  const where = session?.machine_name ?? 'its computer';
  return <p className="why-not" role="alert">The runner on {where} did not answer for {who}. <small className="refusal-name">{entry.refusal}: {entry.reason}</small></p>;
}

/** The connections in words, for a reader who does not follow the lines. */
function Connections({ graph }: { graph: SessionGraph }) {
  return <section className="canvas-connections" aria-label="Connections"><h4>Connections ({graph.edges.length})</h4>
    <ul>{graph.edges.map((edge) => <li key={edge.id} data-kind={edge.kind}>{graph.nodes.find((node) => node.id === edge.from)?.title} → {graph.nodes.find((node) => node.id === edge.to)?.title}: {edge.label}</li>)}</ul>
    {!graph.edges.length ? <p>No team memberships or grants connecting these sessions were returned.</p> : null}
  </section>;
}

/**
 * The surface with whatever message connections have been read. It is one Canvas in one place whether the messages are
 * still being read, were refused, or have arrived, so a terminal that is open stays open across that change.
 */
function Whole({ graph, messages, keeping, board }: { graph: SessionGraph; messages: Load<MessageRead>; keeping: Keeping; board: Load<Board> }) {
  const [later, setLater] = useState<MessageRead | null>(null);
  const read = later ?? (messages.status === 'ok' ? messages.data : null);
  const whole = read ? withMessages(graph, read.messages) : graph;
  return <>
    <div className="session-canvas-strip">
      {graph.notices.map((notice) => <p className="note" role="status" key={notice}>{notice}</p>)}
      {graph.unanswered.map((entry) => <Unanswered key={entry.session} graph={graph} entry={entry} />)}
    </div>
    {/* The connections in words are in the panel that slides out over the canvas: there for a reader who wants them, never a line across the page. */}
    <Canvas graph={whole} keeping={keeping} board={board} connections={<>
      {messages.status === 'loading' ? <p role="status">Reading message connections…</p> : messages.status === 'refused' ? <p className="why-not" role="status">Message connections unavailable: {messages.refused.refusal.refusal}: {messages.refused.refusal.reason}</p> : null}
      {read ? <section className="canvas-about" aria-label="Message connections"><MessageConnections value={read} change={setLater} /></section> : null}
      <Connections graph={whole} />
    </>} />
  </>;
}

function Operations({ keeping }: { keeping: Keeping }) {
  const [search, setSearch] = useSearchParams();
  const { agent } = useParams();
  const [choice, setChoice] = useState<OperationsChoice>(keeping.operations);
  const [failure, setFailure] = useState<string | null>(null);
  const sending = useRef<Promise<void>>(Promise.resolve());
  const asked = search.get('view');
  const proxy = asked === 'proxy';
  const view = asked === 'canvas' || asked === 'sessions' || asked === 'tree' ? asked : agent || search.has('layout') || search.has('panel') ? 'canvas' : choice.view;
  const put = (next: OperationsChoice) => {
    setChoice(next);
    // Changes are kept in order so a slower write cannot replace a newer choice.
    sending.current = sending.current.then(() => keepOperations(next)).then(() => setFailure(null), (error: unknown) => setFailure(said(error)));
  };
  return <>
    <nav className="operations-swap" aria-label="Operations views">
      {(['tree', 'canvas', 'sessions'] as const).map((each) => <a key={each} href={'#/canvas?view=' + each} aria-current={!proxy && view === each ? 'page' : undefined}
        onClick={(event) => {
          event.preventDefault();
          if (!proxy && view === each) return;
          setSearch((now) => { const next = new URLSearchParams(now); next.set('view', each); return next; }, { replace: true });
          put({ ...choice, view: each });
        }}>{each === 'tree' ? 'Network' : each === 'canvas' ? 'Canvas' : 'Sessions'}</a>)}
      <a href={proxyHref({})} aria-current={proxy ? 'page' : undefined}>Proxy</a>
    </nav>
    {failure ? <p className="why-not" role="alert">Your operations view was not kept on the service. {failure}</p> : null}
    {keeping.why ? <p className="why-not" role="status">Your operations view could not be read from the service. {keeping.why}</p> : null}
    {proxy ? <ProxyOperations /> : view === 'tree' ? <NetworkTree agent={agent} selected={search.get('seat') ?? choice.selected} select={(selected) => {
      setSearch((now) => { const next = new URLSearchParams(now); next.set('seat', selected); return next; }, { replace: true });
      put({ ...choice, selected });
    }} paneWidth={choice.pane_width} resize={(pane_width) => put({ ...choice, pane_width })} />
      : view === 'sessions' ? <div className="canvas-side"><RunningList /><Seats /></div> : <CanvasOperations />}
  </>;
}

function ProxyOperations() {
  const board = useLive(readBoard, 'canvas-board');
  return <ProxyView board={board} />;
}

function CanvasOperations() {
  const load = useLive(readSessionGraph, 'session-canvas');
  const messages = useLoad(firstMessagePage, 'canvas-message-edges');
  const board = useLive(readBoard, 'canvas-board');
  const latest = useLoad(readKeeping, 'canvas-arrangement');
  if (latest.status === 'loading') return <p role="status">Reading your canvas arrangement…</p>;
  if (latest.status === 'refused') return <p className="why-not" role="alert">{said(latest.refused)}</p>;
  const keeping = latest.data;
  return <div className="canvas-side">
    {load.status === 'refused' ? <Act symbol="retry" name="Reconnect" word="Reconnect" onClick={refreshLive} /> : null}
    {(load.status === 'ok' && load.data.nodes.some((node) => node.session)) || (load.status === 'refused' && load.refused.status === 401) ? null : <><RunningList /><Seats /></>}
    <Gate load={load} title="Agent canvas" ok={(graph) => graph.nodes.some((node) => node.session) ? <Whole graph={graph} messages={messages} keeping={keeping} board={board} /> : <>
      {graph.notices.map((notice) => <p className="note" role="status" key={notice}>{notice}</p>)}
      {graph.unanswered.map((entry) => <Unanswered key={entry.session} graph={graph} entry={entry} />)}
    </>} />
  </div>;
}

export function SessionCanvas() {
  const keeping = useLoad(readKeeping, 'canvas-kept');
  return <div className="page fill session-canvas-page">
    <div className="head"><h1>Operations</h1></div>
    {keeping.status === 'ok' ? <Operations keeping={keeping.data} /> : keeping.status === 'refused' ? <p className="why-not" role="alert">{said(keeping.refused)}</p> : <p role="status">Reading your operations view…</p>}
  </div>;
}
