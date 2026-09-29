/** A keyboard-accessible canvas of permitted sessions and recorded connections, with one live terminal open at a time. */
import { useLayoutEffect, useRef, useState } from 'react';
import { useLoad } from '../../api';
import { Gate } from '../signin/Gate';
import { Terminal } from './Terminal';
import { readSessionGraph, withMessages } from './session-graph';
import type { SessionGraph } from './session-graph';
import { firstMessagePage } from './message-connections';
import type { MessageRead } from './message-connections';
import { MessageConnections } from './MessageConnections';
import './session-canvas.css';

interface Line { id: string; path: string; kind: string; stands: boolean }

function Canvas({ graph }: { graph: SessionGraph }) {
  const [selected, setSelected] = useState<string | null>(null);
  const [lines, setLines] = useState<Line[]>([]);
  const space = useRef<HTMLDivElement>(null);
  const nodes = useRef(new Map<string, HTMLElement>());
  useLayoutEffect(() => {
    const element = space.current;
    if (!element) return;
    const measure = () => {
      const origin = element.getBoundingClientRect();
      const next = graph.edges.flatMap((edge) => {
        const from = nodes.current.get(edge.from)?.getBoundingClientRect();
        const to = nodes.current.get(edge.to)?.getBoundingClientRect();
        if (!from || !to) return [];
        const x = from.right - origin.left;
        const y = from.top + from.height / 2 - origin.top;
        const endX = to.left - origin.left;
        const endY = to.top + to.height / 2 - origin.top;
        const middle = (x + endX) / 2;
        return [{ id: edge.id, kind: edge.kind, stands: edge.stands, path: `M ${x} ${y} C ${middle} ${y}, ${middle} ${endY}, ${endX} ${endY}` }];
      });
      setLines((previous) => JSON.stringify(previous) === JSON.stringify(next) ? previous : next);
    };
    const observer = new ResizeObserver(measure);
    observer.observe(element);
    for (const node of nodes.current.values()) observer.observe(node);
    measure();
    return () => observer.disconnect();
  }, [graph, selected]);
  return <>
    <div className="session-canvas-scroll" role="region" aria-label="Agent connection canvas" tabIndex={0}>
      <div className="session-canvas" ref={space}>
        <svg className="session-canvas-lines" aria-hidden="true">{lines.map((line) => <path key={line.id} d={line.path} data-kind={line.kind} data-standing={line.stands} />)}</svg>
        {(['teams', 'sessions', 'resources'] as const).map((column) => <section className={'session-canvas-column ' + column} key={column} aria-label={column}>
          <h2>{column === 'sessions' ? 'Running agents' : column === 'teams' ? 'Teams and senders' : 'Resources and recipients'}</h2>
          {graph.nodes.filter((node) => node.column === column).map((node) => <article className="session-canvas-node" key={node.id} ref={(element) => { if (element) nodes.current.set(node.id, element); else nodes.current.delete(node.id); }}>
            <h3>{node.title}</h3><p className="note">{node.detail}</p>
            {node.session ? <>
              <p className="note">{graph.unanswered.some((entry) => entry.session === node.session?.session) ? 'Runner did not answer; current state unknown' : node.session.shown === 'running' ? 'Running' : 'Starting, not yet confirmed'}</p>
              <button className="btn" aria-expanded={selected === node.id} onClick={() => setSelected(selected === node.id ? null : node.id)}>{selected === node.id ? 'Close terminal view' : 'Open terminal'}</button>
              {selected === node.id ? <Terminal key={node.session.session} session={node.session.session} agent={node.session.agent} /> : null}
            </> : null}
          </article>)}
        </section>)}
      </div>
    </div>
    <details className="canvas-connections"><summary>Read the connections ({graph.edges.length})</summary>
      <ul>{graph.edges.map((edge) => <li key={edge.id} data-kind={edge.kind}>{graph.nodes.find((node) => node.id === edge.from)?.title} → {graph.nodes.find((node) => node.id === edge.to)?.title}: {edge.label}</li>)}</ul>
      {!graph.edges.length ? <p>No team memberships or grants connecting these sessions were returned.</p> : null}
    </details>
  </>;
}

function MessageCanvas({ graph, first }: { graph: SessionGraph; first: MessageRead }) {
  const [messages, setMessages] = useState(first);
  return <><MessageConnections value={messages} change={setMessages} /><Canvas graph={withMessages(graph, messages.messages)} /></>;
}

export function SessionCanvas() {
  const [revision, setRevision] = useState(0);
  const load = useLoad(readSessionGraph, 'session-canvas:' + revision);
  const messages = useLoad(firstMessagePage, 'canvas-message-edges:' + revision);
  return <div className="page session-canvas-page">
    <h1>Agent canvas</h1>
    <p className="sub">Open an agent’s terminal beside its team memberships and recorded grants. Closing a view leaves the process running.</p>
    <div className="actions"><a className="btn" href="#/runtime">Session list</a><button className="btn" onClick={() => setRevision((value) => value + 1)}>Refresh connections</button></div>
    <Gate load={load} title="Agent canvas" ok={(graph) => <>
      {graph.notices.map((notice) => <p className="note" role="status" key={notice}>{notice}</p>)}
      {graph.unanswered.map((entry) => <p className="why-not" role="alert" key={entry.session}>{entry.session}: {entry.refusal}: {entry.reason}</p>)}
      {messages.status === 'loading' ? <p role="status">Reading message connections…</p> : messages.status === 'refused' ? <p className="why-not" role="status">Message connections unavailable: {messages.refused.refusal.refusal}: {messages.refused.refusal.reason}</p> : null}
      {graph.nodes.some((node) => node.session) ? messages.status === 'ok' ? <MessageCanvas key={revision} graph={graph} first={messages.data} /> : <Canvas graph={graph} /> : <p>No running sessions were returned.</p>}
      <p className="note">Team membership does not grant access. Dashed grant connections no longer stand. Terminal actions are checked by the service each time.</p>
    </>} />
  </div>;
}
