import { useEffect, useLayoutEffect, useRef, useState } from 'react';
import type { MouseEvent } from 'react';
import { CONCEPTS } from './concepts';
import type { Concept } from './concepts';
import { useShell } from './ShellContext';
import { Act } from './Act';

interface Mark {
  concept: Concept;
  x: number;
  y: number;
}

/** The first on-screen element of every concept, numbered where it stands. */
function measure(): Mark[] {
  const vw = innerWidth;
  const vh = innerHeight;
  const placed: [number, number][] = [];
  const marks: Mark[] = [];
  for (const concept of CONCEPTS) {
    const el = [...document.querySelectorAll(concept.sel)].find((e) => {
      const r = e.getBoundingClientRect();
      return r.width > 0 && r.bottom > 0 && r.top < vh && r.left < vw && !e.closest('.xlayer');
    });
    if (!el) continue;
    const r = el.getBoundingClientRect();
    let x = Math.max(4, r.left - 8);
    const y = Math.max(4, r.top - 8);
    while (placed.some(([a, b]) => Math.abs(a - x) < 20 && Math.abs(b - y) < 20)) x += 22;
    placed.push([x, y]);
    marks.push({ concept, x, y });
  }
  return marks;
}

/**
 * Explain mode: numbers what is on screen. The layer swallows the click that
 * dismisses it, so nothing underneath is pressed; Escape exits and focus
 * returns to where it was (conformance 9.2).
 */
export function Explain() {
  const shell = useShell();
  const [marks, setMarks] = useState<Mark[]>([]);
  const [card, setCard] = useState<{ c: Concept; left: number; top: number } | null>(null);
  const layer = useRef<HTMLElement>(null);

  useLayoutEffect(() => setMarks(measure()), []);
  useEffect(() => {
    const first = layer.current?.querySelector<HTMLElement>('.xm');
    first?.focus();
  }, [marks.length]);
  useEffect(() => {
    // Layout is re-measured on its own signal, never on a timer: a
    // ResizeObserver on every element a number could stand on and on the
    // page itself, and one animation frame after the page's content
    // changes, scrolls or the window resizes, which move marks without
    // resizing what they stand on.
    let frame = 0;
    const remeasure = () => setMarks(measure());
    const resizes = typeof ResizeObserver === 'undefined' ? null : new ResizeObserver(remeasure);
    const observe = () => {
      if (!resizes) return;
      resizes.disconnect();
      resizes.observe(document.documentElement);
      for (const concept of CONCEPTS) {
        for (const el of document.querySelectorAll(concept.sel)) if (!el.closest('.xlayer')) resizes.observe(el);
      }
    };
    const nextFrame = () => {
      if (frame) return;
      frame = requestAnimationFrame(() => {
        frame = 0;
        observe();
        remeasure();
      });
    };
    const changes = new MutationObserver((records) => {
      if (records.some((r) => !(r.target instanceof Element && r.target.closest('.xlayer')))) nextFrame();
    });
    observe();
    changes.observe(document.body, { childList: true, subtree: true, characterData: true });
    addEventListener('scroll', nextFrame, true);
    addEventListener('resize', nextFrame);
    return () => {
      cancelAnimationFrame(frame);
      resizes?.disconnect();
      changes.disconnect();
      removeEventListener('scroll', nextFrame, true);
      removeEventListener('resize', nextFrame);
    };
  }, []);

  const swallow = (event: MouseEvent) => {
    event.stopPropagation();
    event.preventDefault();
    shell.explainOff();
  };
  const hover = (c: Concept, target: HTMLElement) => {
    const r = target.getBoundingClientRect();
    setCard({ c, left: Math.min(innerWidth - 280, r.right + 8), top: Math.min(innerHeight - 90, r.top) });
  };

  return (
    <section className="xlayer" aria-label="Explain mode" id="xlayer" ref={layer} onMouseLeave={() => setCard(null)}>
      <div data-xabsorb="" style={{ position: 'absolute', inset: 0, pointerEvents: 'auto', background: 'rgba(0,0,0,.18)' }} onClick={swallow} />
      {marks.map((m, i) => (
        <button
          key={m.concept.id}
          className="xm"
          style={{ left: m.x, top: m.y }}
          data-xm={m.concept.id}
          aria-label={'Explain: ' + m.concept.t}
          onMouseOver={(e) => hover(m.concept, e.currentTarget)}
          onFocus={(e) => hover(m.concept, e.currentTarget)}
          onClick={(e) => {
            e.stopPropagation();
            shell.explainOff();
            shell.showHelp(m.concept.id);
          }}
        >
          {i + 1}
        </button>
      ))}
      <div className="xbar">
        {marks.length} things explained · click a number{' '}
        <Act symbol="approve" name="Done" hint="Done (Esc)" word="Done" data-xoff="" onClick={swallow} />
      </div>
      <div className="xcard" id="xcard" style={card ? { display: 'block', left: card.left, top: card.top } : { display: 'none' }}>
        {card ? (
          <>
            <b>{card.c.t}</b>
            {card.c.s}
          </>
        ) : null}
      </div>
    </section>
  );
}
