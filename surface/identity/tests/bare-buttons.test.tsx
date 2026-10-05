import { readdirSync, readFileSync } from 'node:fs';
import { join, relative, sep } from 'node:path';
import { describe, expect, it } from 'vitest';
import { STANDING, STOOD_AT_THE_START } from './bare-buttons.standing';

const SRC = join(__dirname, '..', 'src');
const CONTROL = 'shell/Act.tsx';

function sources(dir: string): string[] {
  return readdirSync(dir, { withFileTypes: true }).flatMap((entry) => {
    const path = join(dir, entry.name);
    if (entry.isDirectory()) return sources(path);
    return entry.name.endsWith('.tsx') ? [path] : [];
  });
}

/** The bare `<button` elements written in each source file, by its path under src. */
function standing(): Record<string, number> {
  const found: Record<string, number> = {};
  for (const path of sources(SRC)) {
    const count = readFileSync(path, 'utf8').match(/<button[\s>]/g)?.length ?? 0;
    if (count > 0) found[relative(SRC, path).split(sep).join('/')] = count;
  }
  return found;
}

/** A bare button says a state, or is a thing chosen from a list, or is one of the drawn controls a screen owns.
 *  Anything else is an act, and an act is an `Act`. */
const STATE = /aria-pressed=|aria-expanded=|aria-checked=|role="(menuitem|tab|radio)"/;
/** A list entry: its words are the thing chosen, a name from data and never a verb. */
const CHOICE = /data-choice=""/;
/** The drawn controls a screen owns, each already a symbol with its own spoken name: the canvas's bar grip, its
 *  symbol buttons, colour swatches, the cross on a note, a line and a widget, a widget's turn; explain mode's
 *  numbered markers; the rail's places. */
const OWN = /className=(?:"|\{')(?:canvas-bar-grip|canvas-symbol|canvas-swatch|canvas-mark-remove|canvas-widget-turn|canvas-widget-remove|xm|rb)\b/;

/** Every bare button outside the control, as its source from `<button` to its end or to the next button, with
 *  those that are acts picked out. A button written `<button ... />` ends where the next one starts. */
function bare(): { seen: number; acts: string[] } {
  const acts: string[] = [];
  let seen = 0;
  for (const path of sources(SRC)) {
    const file = relative(SRC, path).split(sep).join('/');
    if (file === CONTROL) continue;
    const source = readFileSync(path, 'utf8');
    const starts = [...source.matchAll(/<button[\s>]/g)].map((match) => match.index);
    starts.forEach((start, index) => {
      const close = source.indexOf('</button>', start);
      const next = starts[index + 1] ?? source.length;
      const button = source.slice(start, Math.min(next, close < 0 ? next : close));
      seen += 1;
      if (!STATE.test(button) && !CHOICE.test(button) && !OWN.test(button)) acts.push(file + ': ' + button.replace(/\s+/g, ' ').slice(0, 90));
    });
  }
  return { seen, acts };
}

describe('Bare buttons', () => {
  it('are never acts: each says a state, is a thing chosen from a list, or is a drawn control its screen owns', () => {
    const { seen, acts } = bare();
    expect(acts).toEqual([]);
    // The rule read every bare button the table counts, so none slipped past it unread.
    expect(seen).toBe(Object.values(STANDING).reduce((sum, count) => sum + count, 0));
  });


  it('stand only where the table says, and the act control holds the one button every act is drawn with', () => {
    const { [CONTROL]: control, ...outside } = standing();
    expect(control).toBe(1);
    // A file with more than its number has a new bare button: write an `Act`. A file with fewer has had
    // buttons moved to the control: lower its number here, in the same commit.
    expect(outside).toEqual(STANDING);
  });

  it('never number more than stood when the count began', () => {
    const total = Object.values(STANDING).reduce((sum, count) => sum + count, 0);
    expect(total).toBeLessThanOrEqual(STOOD_AT_THE_START);
    expect(Object.keys(STANDING)).not.toContain(CONTROL);
    expect(Object.values(STANDING).every((count) => Number.isInteger(count) && count > 0)).toBe(true);
  });
});
