import { useEffect, useLayoutEffect, useRef, useState } from 'react';
import type { MouseEvent } from 'react';
import { useLocation } from 'react-router';
import { Act } from '../Act';
import { useShell } from '../ShellContext';
import { ANY_PAGE, PAGES, RAIL_REGION, pageKeyOf } from './pages';
import type { Region } from './pages';
import { SHELL_SHORTCUTS, goShortcuts } from './shortcuts';
import './help.css';

interface Placed {
  n: number;
  region: Region;
  box: Box;
}

interface Box {
  left: number;
  top: number;
  width: number;
  height: number;
}

/** Whether any of a box is inside the window. */
const onScreen = (b: DOMRect, w: number, h: number): boolean => b.width > 0 && b.height > 0 && b.bottom > 0 && b.right > 0 && b.top < h && b.left < w;

/** The first on-screen element of each region, numbered in the page's order; regions not on screen are skipped. */
function measure(regions: readonly Region[]): Placed[] {
  const out: Placed[] = [];
  for (const region of regions) {
    const el = [...document.querySelectorAll(region.sel)].find((e) => !e.closest('#helpLayer') && onScreen(e.getBoundingClientRect(), innerWidth, innerHeight));
    if (!el) continue;
    const { left, top, width, height } = el.getBoundingClientRect();
    out.push({ n: out.length + 1, region, box: { left, top, width, height } });
  }
  return out;
}

/**
 * The page help: the page dims, each region of the page on screen is outlined and carries a number, and a panel down
 * the right lists every number's note in order, then the keys that work here. Nothing floats beside a region, so the
 * notes never cover each other or the page, however many a screen has; the panel scrolls. ? (outside a field) or ⌘/
 * (anywhere) shows and hides it; Esc, the close button or a click on the dim closes it, and the click never presses
 * what is underneath. Pressing a note scrolls its region into view. "Full help" opens the help dock on this page's
 * sheet.
 */
export function PageHelp() {
  const shell = useShell();
  const { pathname } = useLocation();
  const help = PAGES[pageKeyOf(pathname)] ?? ANY_PAGE;
  const [placed, setPlaced] = useState<Placed[]>([]);
  const [layout, setLayout] = useState(0);
  const panel = useRef<HTMLDivElement>(null);

  useLayoutEffect(() => {
    setPlaced(measure([...help.regions, RAIL_REGION]));
  }, [help, layout]);

  // Layout is re-measured on its own signals, never on a timer: a ResizeObserver on the page and on every element a
  // region stands on, and one animation frame after the screen's content changes (a list arriving), it scrolls or
  // the window resizes, which move regions without resizing them.
  useEffect(() => {
    let frame = 0;
    const remeasure = () => setLayout((n) => n + 1);
    const resizes = typeof ResizeObserver === 'undefined' ? null : new ResizeObserver(remeasure);
    const observe = () => {
      if (!resizes) return;
      resizes.disconnect();
      resizes.observe(document.documentElement);
      for (const region of [...help.regions, RAIL_REGION]) {
        for (const el of document.querySelectorAll(region.sel)) if (!el.closest('#helpLayer')) resizes.observe(el);
      }
    };
    const nextFrame = () => {
      if (frame) return;
      frame = requestAnimationFrame(() => { frame = 0; observe(); remeasure(); });
    };
    const changes = new MutationObserver((records) => {
      if (records.some((r) => !(r.target instanceof Element && r.target.closest('#helpLayer')))) nextFrame();
    });
    observe();
    changes.observe(document.body, { childList: true, subtree: true, characterData: true });
    addEventListener('resize', nextFrame);
    addEventListener('scroll', nextFrame, true);
    return () => {
      cancelAnimationFrame(frame);
      resizes?.disconnect();
      changes.disconnect();
      removeEventListener('resize', nextFrame);
      removeEventListener('scroll', nextFrame, true);
    };
  }, [help]);

  useEffect(() => { panel.current?.focus(); }, []);

  const dismiss = (event: MouseEvent) => {
    event.stopPropagation();
    event.preventDefault();
    shell.explainOff();
  };
  const full = () => {
    shell.explainOff();
    shell.showHelp(help.topic || null);
  };
  // A note pressed brings its region into view; the layer stays up and the outlines follow the scroll.
  const show = (region: Region) => {
    const el = [...document.querySelectorAll(region.sel)].find((e) => !e.closest('#helpLayer'));
    el?.scrollIntoView({ block: 'center', behavior: 'smooth' });
  };

  return (
    <section id="helpLayer" className="help-layer" aria-label={'Help: ' + help.title}>
      <div className="help-dim" data-help-dim="" onClick={dismiss} />
      {placed.map(({ n, region, box }) => (
        <div key={n + region.sel} className="help-reg" data-help-region={region.title} style={{ left: box.left, top: box.top, width: box.width, height: box.height }}>
          <span className="help-n">{n}</span>
        </div>
      ))}
      <div className="help-keys" role="dialog" aria-label={'Help for ' + help.title} tabIndex={-1} ref={panel}>
        <div className="help-keys-head">
          <b>{help.title}</b>
          <Act symbol="close" name="Close help" hint="Close help (Esc)" className="small quiet" data-help-close="" onClick={shell.explainOff} />
        </div>
        {help === ANY_PAGE ? <p className="dim">This page has no numbered notes; Full help explains every screen.</p> : null}
        {placed.length ? (
          <ol className="help-notes" aria-label={'Notes for ' + help.title}>
            {placed.map(({ n, region }) => (
              <li key={n + region.sel} data-help-note={n}>
                <button type="button" className="help-note" onClick={() => show(region)}>
                  <div className="help-call-t"><span className="help-n">{n}</span>{region.title}</div>
                  <div className="help-call-d">{region.text}</div>
                </button>
              </li>
            ))}
          </ol>
        ) : null}
        <div className="help-kt-head">Keys</div>
        <dl className="help-kt">
          {[...help.keys, ...SHELL_SHORTCUTS, ...goShortcuts()].map(([k, d]) => (
            <div key={k} className="help-kt-row"><dt>{k}</dt><dd>{d}</dd></div>
          ))}
        </dl>
        <div className="help-keys-foot">
          <Act symbol="help" name="Full help" hint="Every topic: getting started, how each screen works, and what to do when something isn’t working" word="Full help" tone="primary" data-help-full="" onClick={full} />
        </div>
      </div>
    </section>
  );
}
