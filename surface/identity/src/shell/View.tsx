/** The terminals the person is watching, kept in the corner of every page but Running, which shows them itself: one small live picture at a time with the others as tabs, dragged by its corner to any size, and opened into a multiplexer of real terminals that fills the screen. Hiding it keeps the set; the sessions keep running either way. */
import { useEffect, useRef } from 'react';
import { useLocation } from 'react-router';
import type { KeyboardEvent, PointerEvent } from 'react';
import { PEEK } from '../features/runtime/GpuTerminal';
import { Peek } from '../features/runtime/Peek';
import { Terminal } from '../features/runtime/Terminal';
import { useShell } from './ShellContext';
import type { Watched } from './ShellContext';
import '../styles/view.css';

function Tabs({ watched, viewing, choose, remove }: { watched: Watched[]; viewing: string | null; choose: (session: string) => void; remove: (session: string) => void }) {
  return <div className="view-tabs" role="tablist" aria-label="Watched agents">
    {watched.map((entry) => <span className="view-tab" key={entry.session} data-on={entry.session === viewing || undefined}>
      <button type="button" role="tab" aria-selected={entry.session === viewing} onClick={() => choose(entry.session)}>{entry.name}</button>
      <button type="button" className="view-x" title={'Stop watching ' + entry.name + '; it keeps running'} aria-label={'Stop watching ' + entry.name} onClick={() => remove(entry.session)}>×</button>
    </span>)}
  </div>;
}

function Small() {
  const shell = useShell();
  const current = shell.watched.find((entry) => entry.session === shell.viewing) ?? shell.watched[0];
  const drag = useRef<{ x: number; width: number } | null>(null);
  const real = shell.viewWidth >= PEEK.width;
  const onDown = (event: PointerEvent<HTMLButtonElement>) => {
    drag.current = { x: event.clientX, width: shell.viewWidth };
    event.currentTarget.setPointerCapture(event.pointerId);
  };
  const onMove = (event: PointerEvent<HTMLButtonElement>) => {
    if (!drag.current) return;
    shell.setViewWidth(drag.current.width + (drag.current.x - event.clientX));
  };
  const onUp = (event: PointerEvent<HTMLButtonElement>) => {
    drag.current = null;
    event.currentTarget.releasePointerCapture(event.pointerId);
  };
  if (!current) return null;
  const height = Math.round(PEEK.height * shell.viewWidth / PEEK.width);
  return <aside className="view view-small" aria-label="Watched agents" style={{ width: shell.viewWidth }}>
    <button type="button" className="view-grip" title="Drag to resize" aria-label="Resize the view" onPointerDown={onDown} onPointerMove={onMove} onPointerUp={onUp} onPointerCancel={onUp} />
    <div className="view-head">
      <Tabs watched={shell.watched} viewing={current.session} choose={shell.setViewing} remove={shell.unwatch} />
      <span className="view-acts">
        <button type="button" className="view-act" onClick={() => shell.setViewState('open')}>Open</button>
        <button type="button" className="view-act" title="Hide the view; the agents keep running" onClick={() => shell.setViewState('hidden')}>Hide</button>
      </span>
    </div>
    {real
      ? <div className="view-real" style={{ height }}><Terminal key={current.session} session={current.session} agent={current.agent} bare /></div>
      : <button type="button" className="view-picture" title="Open the terminals" onClick={() => shell.setViewState('open')}>
        <Peek key={current.session} session={current.session} name={current.name} machine={current.machine} width={shell.viewWidth} />
      </button>}
  </aside>;
}

function Open() {
  const shell = useShell();
  const box = useRef<HTMLDivElement>(null);
  useEffect(() => { box.current?.focus(); }, []);
  const onKey = (event: KeyboardEvent<HTMLDivElement>) => {
    if (event.key === 'Escape') { event.stopPropagation(); shell.setViewState('small'); }
  };
  const count = Math.min(shell.watched.length, 4);
  const shown = shell.watched.slice(0, 4);
  return <div className="view view-open" role="dialog" aria-label="Watched agents" tabIndex={-1} ref={box} onKeyDown={onKey}>
    <div className="view-head">
      <Tabs watched={shell.watched} viewing={shell.viewing} choose={shell.setViewing} remove={shell.unwatch} />
      <span className="view-acts">
        <button type="button" className="view-act" onClick={() => shell.setViewState('small')}>Back to the page</button>
      </span>
    </div>
    <div className="view-panes" data-n={count}>
      {shown.map((entry) => <div className="view-pane" key={entry.session} data-on={entry.session === shell.viewing || undefined}>
        <div className="view-pane-name">{entry.name}{entry.machine ? <span className="view-machine"> on {entry.machine}</span> : null}</div>
        <Terminal session={entry.session} agent={entry.agent} bare />
      </div>)}
    </div>
    {shell.watched.length > 4 ? <p className="view-more">The first four are shown; stop watching one to bring the next in.</p> : null}
  </div>;
}

function Hidden() {
  const shell = useShell();
  return <button type="button" className="view view-hidden" title="Show the agents you are watching" onClick={() => shell.setViewState('small')}>
    {shell.watched.length === 1 ? '1 agent watched' : shell.watched.length + ' agents watched'}
  </button>;
}

export function View() {
  const shell = useShell();
  const { pathname } = useLocation();
  if (!shell.watched.length) return null;
  if (shell.viewState === 'open') return <Open />;
  // The Running page shows the terminals itself; the corner picture would sit on top of the one that is open.
  if (pathname === '/runtime' || pathname.startsWith('/runtime/')) return null;
  if (shell.viewState === 'hidden') return <Hidden />;
  return <Small />;
}
