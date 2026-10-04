/**
 * What hovers over the canvas and never takes it over: a small dock at its bottom left to find an agent and to draw a
 * box, a note or a line, and one panel that slides out from the right edge for the saved layouts and for the
 * connections in words. Neither blocks the surface; the canvas stays live under both.
 */
import { useState } from 'react';
import type { ReactNode } from 'react';
import { api } from '../../api';
import { entries } from '../people/directory';
import type { Entry } from '../people/directory';
import { clock } from '../file/time';
import type { SessionGraph } from './session-graph';
import { said } from './canvas-kept';
import type { Keeping, SavedLayout } from './canvas-kept';
import type { Arrangement } from './canvas-marks';

export type Tool = 'box' | 'line' | null;
export type Panel = 'layouts' | 'connections' | null;

/** Find an agent by name: one that is running goes to its window here, one that is not goes to its own page. */
function Find({ graph, show }: { graph: SessionGraph; show: (node: string) => void }) {
  const [query, setQuery] = useState('');
  const [agents, setAgents] = useState<Entry[] | null>(null);
  const [refused, setRefused] = useState<string | null>(null);
  const read = () => {
    if (agents) return;
    api.people().then((answer) => { setAgents(entries(answer).filter((each) => each.kind === 'agent')); setRefused(null); },
      (error: unknown) => setRefused(said(error)));
  };
  const running = graph.nodes.flatMap((node) => node.session ? [{ node: node.id, agent: node.session.agent, name: node.title, where: node.session.machine_name ?? node.session.machine }] : []);
  const idle = (agents ?? []).filter((each) => each.state !== 'retired' && !running.some((live) => live.agent === each.id));
  const wanted = (name: string) => name.toLowerCase().includes(query.trim().toLowerCase());
  const [live, rest] = [running.filter((each) => wanted(each.name)), idle.filter((each) => wanted(each.display_name))];
  const first = live[0] ? '#/canvas' + (live[0].agent ? '/' + encodeURIComponent(live[0].agent) : '') : rest[0] ? '#/file/' + rest[0].id : null;
  return <div className="canvas-find">
    <input type="search" aria-label="Find an agent" placeholder="Find an agent" value={query} onFocus={read} onChange={(event) => setQuery(event.target.value)}
      onKeyDown={(event) => { if (event.key === 'Enter' && first) { if (live[0]) show(live[0].node); location.hash = first; event.currentTarget.blur(); } }} />
    <div className="canvas-find-list" role="listbox" aria-label="Agents">
      {live.map((each) => <a key={each.node} role="option" aria-selected={false} href={'#/canvas' + (each.agent ? '/' + encodeURIComponent(each.agent) : '')} data-find="running" onClick={() => show(each.node)}>
        <span><span className="dot s-active" />{each.name}</span><span className="sec">Running on {each.where}</span></a>)}
      {rest.map((each) => <a key={each.id} role="option" aria-selected={false} href={'#/file/' + each.id} data-find="idle">
        <span>{each.display_name}</span><span className="sec">Not running</span></a>)}
      {refused ? <p className="why-not" role="alert">The agents that are not running could not be read. <small className="refusal-name">{refused}</small></p> : null}
      {!live.length && !rest.length && !refused ? <p className="dim">{agents ? 'No agent by that name.' : 'Reading the agents…'}</p> : null}
    </div>
  </div>;
}

export function CanvasDock({ graph, show, tool, setTool, picking, addNote, panel, setPanel, edges, unsent }: {
  graph: SessionGraph; show: (node: string) => void; tool: Tool; setTool: (tool: Tool) => void; picking: boolean;
  addNote: () => void; panel: Panel; setPanel: (panel: Panel) => void; edges: number; unsent: string | null;
}) {
  const press = (name: Exclude<Tool, null>) => () => setTool(tool === name ? null : name);
  const slide = (name: Exclude<Panel, null>) => () => setPanel(panel === name ? null : name);
  return <div className="canvas-dock" role="toolbar" aria-label="Canvas tools">
    <Find graph={graph} show={show} />
    <button type="button" className="btn" data-act="draw-box" aria-pressed={tool === 'box'} title="Drag on the canvas to draw a box around windows, then label it" onClick={press('box')}>Box</button>
    <button type="button" className="btn" data-act="add-note" title="Put a note on the canvas" onClick={addNote}>Note</button>
    <button type="button" className="btn" data-act="draw-line" aria-pressed={tool === 'line'} title="Press one thing, then another, to draw a line between them" onClick={press('line')}>Line</button>
    <button type="button" className="btn" data-act="layouts" aria-expanded={panel === 'layouts'} onClick={slide('layouts')}>Layouts</button>
    <button type="button" className="btn" data-act="connections" aria-expanded={panel === 'connections'} onClick={slide('connections')}>Connections ({edges})</button>
    {tool ? <span className="canvas-dock-say" role="status">{tool === 'box' ? 'Drag on the canvas to draw the box.' : picking ? 'Now press the thing the line goes to.' : 'Press the thing the line starts from.'} Escape leaves it.</span> : null}
    {unsent ? <span className="why-not" role="alert">The arrangement was not kept on the service. <small className="refusal-name">{unsent}</small></span> : null}
  </div>;
}

/** The saved layouts as one table: each is opened, saved over or removed where it stands, and the last row saves the canvas as it is under a new name. */
function Layouts({ keeping, layouts, save, open, remove }: {
  keeping: Keeping; layouts: SavedLayout[]; save: (name: string) => Promise<void>; open: (layout: Arrangement) => void; remove: (name: string) => Promise<void>;
}) {
  const [name, setName] = useState('');
  const [refused, setRefused] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const doing = (work: Promise<void>, after?: () => void) => {
    setBusy(true);
    work.then(() => { setRefused(null); after?.(); }, (error: unknown) => setRefused(said(error))).finally(() => setBusy(false));
  };
  const wanted = name.trim();
  return <>
    <p className="note">{keeping.where === 'service'
      ? 'Your layouts are yours: kept on the service, the same on any computer you sign in at.'
      : <>Kept in this browser only: the service is not keeping your canvas. <small className="refusal-name">{keeping.why}</small></>}</p>
    <form id="canvas-save-layout" aria-label="Save this layout" onSubmit={(event) => { event.preventDefault(); if (wanted && !busy) doing(save(wanted), () => setName('')); }} />
    <table className="canvas-layouts"><thead><tr><th>Layout</th><th>Saved</th><th>Change</th></tr></thead>
      <tbody>
        {layouts.map((layout) => <tr key={layout.name} data-layout={layout.name}>
          <td>{layout.name}</td><td className="sec">{clock(layout.saved_at)}</td>
          <td className="canvas-layout-acts">
            <button type="button" className="btn" data-act="open-layout" onClick={() => open(layout.arrangement)}>Open</button>
            <button type="button" className="btn" data-act="save-over" disabled={busy} onClick={() => doing(save(layout.name))}>Save over</button>
            <button type="button" className="btn" data-act="remove-layout" disabled={busy} onClick={() => doing(remove(layout.name))}>Remove</button>
          </td></tr>)}
        {layouts.length ? null : <tr className="empty"><td colSpan={3} className="dim">No layout is saved yet.</td></tr>}
      </tbody>
      <tfoot><tr data-add="layout">
        <td colSpan={2}><input form="canvas-save-layout" name="name" aria-label="Name for this layout" placeholder="Name this layout" value={name} onChange={(event) => setName(event.target.value)} /></td>
        <td><button form="canvas-save-layout" type="submit" className="btn primary" disabled={!wanted || busy}>{layouts.some((each) => each.name === wanted) ? 'Save over' : 'Save'}</button></td>
      </tr></tfoot>
    </table>
    {refused ? <p className="why-not" role="alert">That was not done. <small className="refusal-name">{refused}</small></p> : null}
  </>;
}

/** The panel that slides out over the right edge. Both of its parts stay on the page while it is away, so nothing in them is read again when it opens. */
export function CanvasPanel({ panel, close, connections, ...layouts }: {
  panel: Panel; close: () => void; connections: ReactNode;
  keeping: Keeping; layouts: SavedLayout[]; save: (name: string) => Promise<void>; open: (layout: Arrangement) => void; remove: (name: string) => Promise<void>;
}) {
  return <aside className={'canvas-panel' + (panel ? ' open' : '')} aria-label={panel === 'connections' ? 'Connections' : 'Layouts'} aria-hidden={!panel}>
    <header><h2>{panel === 'connections' ? 'Connections' : 'Layouts'}</h2><button type="button" className="btn" data-act="close-panel" tabIndex={panel ? 0 : -1} onClick={close}>Close</button></header>
    <div className="canvas-panel-body" hidden={panel !== 'layouts'}><Layouts {...layouts} /></div>
    <div className="canvas-panel-body" hidden={panel !== 'connections'}>{connections}</div>
  </aside>;
}
