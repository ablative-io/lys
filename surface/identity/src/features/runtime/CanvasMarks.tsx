/** A person's own marks as they are drawn on the surface: boxes with a label, notes, and the lines between things. */
import { useEffect, useRef, useState } from 'react';
import type { PointerEvent } from 'react';
import { SIDES, linkRoute, tint } from './canvas-marks';
import type { Box, Group, Link, Note, Side } from './canvas-marks';

type Press = (event: PointerEvent<HTMLElement>) => void;
interface Held {
  /** Just made by the person, so its words take the keyboard. */
  fresh: boolean;
  /** A line is being drawn: pressing this picks it as an end, and does nothing else. */
  pick: Press | undefined;
  move: Press;
  size: Press;
  /** Starts a line from one edge of this thing, dragged to another thing. */
  link: (side: Side) => Press;
  change: (words: string) => void;
  /** Gives it its next colour. */
  colour: () => void;
  remove: () => void;
}

/** The small dot on a thing's bar that shows its colour and, pressed, gives it the next one. */
const Swatch = ({ on, of }: { on: () => void; of: string }) => <button type="button" className="canvas-swatch" data-act="colour" aria-label={'Change the colour of ' + of} title="Colour" onClick={on} />;

/** A box around windows, with its label on its top edge. Its inside is the surface: only its bar and its corner are held. */
export function GroupBox({ group, fresh, pick, move, size, link, change, colour, remove }: { group: Group } & Held) {
  const [label, setLabel] = useState(group.label);
  useEffect(() => setLabel(group.label), [group.label]);
  // A box is on the surface while it is still being drawn out; its label takes the keyboard when the drawing ends.
  const words = useRef<HTMLInputElement>(null);
  useEffect(() => { if (fresh) words.current?.focus(); }, [fresh]);
  return <section className={'canvas-group' + (pick ? ' picking' : '')} data-group={group.id} aria-label={'Box: ' + (group.label || 'no label yet')}
    style={{ left: group.x, top: group.y, width: group.w, height: group.h, ...tint(group.colour) }}>
    <header className="canvas-group-bar" onPointerDownCapture={pick} onPointerDown={move}>
      <input ref={words} aria-label="Label of this box" placeholder="Label this box" value={label} size={Math.max(14, label.length + 2)}
        onChange={(event) => setLabel(event.target.value)} onBlur={() => { if (label !== group.label) change(label); }}
        onKeyDown={(event) => { if (event.key === 'Enter') event.currentTarget.blur(); }} />
      <Swatch on={colour} of={'the box ' + (group.label || 'with no label')} />
      <button type="button" className="canvas-mark-remove" aria-label={'Remove the box ' + (group.label || 'with no label')} onClick={remove}>×</button>
    </header>
    <span className="session-canvas-grip" aria-hidden="true" onPointerDown={size} />
    <Anchors from={link} />
  </section>;
}

/** A note written on the surface. What is typed is kept when the person leaves the note. */
export function NoteCard({ note, fresh, pick, move, size, link, change, colour, remove }: { note: Note } & Held) {
  const [text, setText] = useState(note.text);
  useEffect(() => setText(note.text), [note.text]);
  return <article className={'canvas-note' + (pick ? ' picking' : '')} data-note={note.id} aria-label="Note"
    style={{ left: note.x, top: note.y, width: note.w, height: note.h, ...tint(note.colour) }} onPointerDownCapture={pick}>
    <header className="canvas-note-bar" onPointerDown={move}>
      <span className="session-canvas-kind">Note</span>
      <Swatch on={colour} of="this note" />
      <button type="button" className="canvas-mark-remove" aria-label="Remove this note" onClick={remove}>×</button>
    </header>
    <textarea aria-label="Note" placeholder="Write a note" value={text} autoFocus={fresh}
      onChange={(event) => setText(event.target.value)} onBlur={() => { if (text !== note.text) change(text); }} />
    <span className="session-canvas-grip" aria-hidden="true" onPointerDown={size} />
    <Anchors from={link} />
  </article>;
}

/** The four dots on a thing's edges that a line is dragged from. */
const DOT: Record<Side, [string, string]> = { top: ['50%', '0'], right: ['100%', '50%'], bottom: ['50%', '100%'], left: ['0', '50%'] };
export function Anchors({ from }: { from: (side: Side) => Press }) {
  return <>{SIDES.map((side) => <span key={side} className="canvas-anchor" data-anchor={side} aria-hidden="true" style={{ left: DOT[side][0], top: DOT[side][1] }} onPointerDown={from(side)} />)}</>;
}

type At = (id: string) => Box | undefined;
const route = (link: Link, at: At) => { const [from, to] = [at(link.from), at(link.to)]; return from && to ? linkRoute(from, to, link) : null; };

/** The lines a person drew, inside the surface's drawing. A line with an end that is not on the surface now is not drawn, and is kept. */
export function LinkLines({ links, at, chosen, choose }: { links: Link[]; at: At; chosen: string | null; choose: (id: string) => void }) {
  return <>{links.flatMap((link) => {
    const line = route(link, at);
    // A line is chosen by pressing it; the Delete key then takes it away.
    return line ? [<path key={link.id} className={'person-line' + (chosen === link.id ? ' chosen' : '')} data-link={link.id} d={line.d} onPointerDown={(event) => { event.stopPropagation(); choose(link.id); }} />] : [];
  })}</>;
}

/** Each drawn line's own small control at its middle, to take the line away. */
export function LinkHandles({ links, at, remove }: { links: Link[]; at: At; remove: (id: string) => void }) {
  return <>{links.flatMap((link) => {
    const line = route(link, at);
    return line ? [<button key={link.id} type="button" className="canvas-mark-remove canvas-link-remove" aria-label="Remove this line"
      style={{ left: line.middle[0], top: line.middle[1] }} onClick={() => remove(link.id)}>×</button>] : [];
  })}</>;
}
