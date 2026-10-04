/**
 * Tidy: everything on the canvas put into rows in one press. A window keeps the widgets and notes whose lines join them
 * to it alone in a column at its right. A box is laid out inside and drawn round what sits in it. At every level teams
 * stand down the left, agents and boxes in rows in the middle, resources down the right, and the widgets and notes that
 * are no one thing's in rows beneath. What sat in a box before sits in it after, and the order things are read in, left
 * to right and top to bottom, is kept. The rows are as long as makes the whole nearest the shape of the page it is
 * seen on. A thing in two boxes that are not one inside the other stays in the smaller.
 */
import { SMALLEST_MARK, standing, within } from './canvas-marks';
import type { Box, Group, Marks, Note, Widget } from './canvas-marks';

/** Which of the three kinds a window is: a team or sender, an agent, a resource or recipient. */
export type Column = 'teams' | 'sessions' | 'resources';
/** The canvas after Tidy: where each window stands, the person's marks where they stand, and the part of the surface the whole takes. */
export interface Tidied { boxes: Record<string, Box>; groups: Group[]; notes: Note[]; widgets: Widget[]; extent: Box }

/** Between a thing and the column at its right (the same as a widget added from a window), and between the things stacked in that column. */
const BESIDE = 48, STACK = 10;
/** Between cards in a column and loose things in a row, between blocks in the rows, and between the columns of a level. */
const CARD = 20, GAP = 40, BETWEEN = 60;
/** A box drawn round what it holds stands clear of it: under its label's bar at the top, and by its edge on every side. */
const BAR = 40, EDGE = 20;

/** A thing on the surface by its name, with where it stands. */
type Thing = Box & { id: string };
/** Things placed together: how much room they take, and where each stands in that room. */
interface Block { w: number; h: number; parts: Thing[] }

const NOTHING: Block = { w: 0, h: 0, parts: [] };
const alone = (thing: Thing): Block => ({ w: thing.w, h: thing.h, parts: [{ ...thing, x: 0, y: 0 }] });
const shifted = (block: Block, x: number, y: number): Thing[] => block.parts.map((part) => ({ ...part, x: part.x + x, y: part.y + y }));

/** Blocks one under another. */
function stacked(blocks: Block[], gap: number): Block {
  let y = 0;
  const parts = blocks.flatMap((block) => { const here = shifted(block, 0, y); y += block.h + gap; return here; });
  return blocks.length ? { w: Math.max(...blocks.map((block) => block.w)), h: y - gap, parts } : NOTHING;
}

/** Blocks side by side, their tops level. */
function beside(blocks: Block[], gap: number): Block {
  let x = 0;
  const parts = blocks.flatMap((block) => { const here = shifted(block, x, 0); x += block.w + gap; return here; });
  return blocks.length ? { w: x - gap, h: Math.max(...blocks.map((block) => block.h)), parts } : NOTHING;
}

/** Blocks in rows no longer than `width`; a block longer than that is a row of its own. */
function rows(blocks: Block[], width: number, gap: number): Block {
  const lines: Block[][] = [];
  let used = 0;
  for (const block of blocks) {
    if (lines.length && used + gap + block.w <= width) { lines[lines.length - 1].push(block); used += gap + block.w; }
    else { lines.push([block]); used = block.w; }
  }
  return stacked(lines.map((line) => beside(line, gap)), gap);
}

/** Blocks in rows, as long as makes the whole nearest `shape`, its width over its height. */
function shaped(blocks: Block[], shape: number, gap: number): Block {
  let best = NOTHING, off = Infinity, width = 0;
  // Each length tried is that of the first few blocks in one row, added up the way a row adds them.
  blocks.forEach((block, index) => {
    width = index ? width + gap + block.w : block.w;
    const tried = rows(blocks, width, gap);
    const away = Math.abs(Math.log(tried.w / tried.h / shape));
    if (away < off) [best, off] = [tried, away];
  });
  return best;
}

/** The order things are read in: row by row from the top, each row from its left. A thing is in a row when it starts above the foot of the row's first. */
function reading<T extends Box>(things: T[]): T[] {
  const read: T[] = [];
  let row: T[] = [];
  const across = () => row.sort((left, right) => left.x - right.x);
  for (const thing of [...things].sort((left, right) => left.y - right.y || left.x - right.x)) {
    if (row.length && thing.y >= row[0].y + row[0].h) { read.push(...across()); row = []; }
    row.push(thing);
  }
  return [...read, ...across()];
}

/**
 * The canvas tidied. `boxes` is every window where it stands, `columns` which kind each is, `here` the name a line's end
 * has on the surface now (an end kept under an agent's name is that agent's window), and `shape` the page's width over
 * its height. A locked widget is arranged with the rest: its lock is against a hand, and Tidy is asked for.
 */
export function tidied(boxes: Record<string, Box>, marks: Marks, columns: Record<string, Column>, here: (id: string) => string, shape: number): Tidied {
  const windows: Thing[] = Object.entries(boxes).map(([id, box]) => ({ ...box, id }));
  const loose: Thing[] = [...marks.notes, ...marks.widgets.map(standing)].map(({ id, x, y, w, h }) => ({ id, x, y, w, h }));
  // The boxes from the smallest: a thing sits in the smallest box its middle is in, and a box only in one after it here, so no two hold each other.
  const ranked = marks.groups.map((group, index) => ({ group, index })).sort((left, right) => left.group.w * left.group.h - right.group.w * right.group.h || left.index - right.index).map((each) => each.group);
  const holder = (thing: Thing): string => ranked.slice(ranked.findIndex((group) => group.id === thing.id) + 1).find((group) => within(group, thing))?.id ?? '';
  // What can have a column of its own: a window or a box. A widget or a note joined by its lines to exactly one of them goes with it.
  const anchors = new Map<string, Thing>([...windows, ...ranked].map((thing) => [thing.id, thing]));
  const anchorOf = (thing: Thing): string | undefined => {
    const ends = new Set(marks.links.flatMap((link) => link.from === thing.id ? [here(link.to)] : link.to === thing.id ? [here(link.from)] : []).filter((end) => anchors.has(end)));
    return ends.size === 1 ? [...ends][0] : undefined;
  };
  const joined = new Map<string, Thing[]>(), held = new Map<string, { anchors: Thing[]; loose: Thing[] }>();
  const level = (id: string) => held.get(id) ?? held.set(id, { anchors: [], loose: [] }).get(id)!;
  for (const anchor of anchors.values()) level(holder(anchor)).anchors.push(anchor);
  for (const thing of loose) {
    const anchor = anchorOf(thing);
    if (anchor === undefined) level(holder(thing)).loose.push(thing);
    else joined.set(anchor, [...joined.get(anchor) ?? [], thing]);
  }

  /** A window or a box with its own column at its right. A box that holds something is drawn round it; an empty one keeps its size. */
  const cluster = (anchor: Thing): Block => {
    const inside = held.has(anchor.id) ? laid(anchor.id) : NOTHING;
    const [w, h] = inside.parts.length ? [Math.max(SMALLEST_MARK[0], inside.w + 2 * EDGE), inside.h + BAR + EDGE] : [anchor.w, anchor.h];
    const core: Block = { w, h, parts: [{ ...anchor, x: 0, y: 0, w, h }, ...shifted(inside, EDGE, BAR)] };
    const column = stacked(reading(joined.get(anchor.id) ?? []).map(alone), STACK);
    return column.parts.length ? { w: w + BESIDE + column.w, h: Math.max(h, column.h), parts: [...core.parts, ...shifted(column, w + BESIDE, 0)] } : core;
  };
  /** One level, the whole canvas or the inside of one box: teams, then the rows of agents and boxes, then resources; the loose things beneath. */
  const laid = (id: string): Block => {
    const here = level(id);
    const of = (column: Column) => reading(here.anchors.filter((anchor) => (columns[anchor.id] ?? 'sessions') === column)).map(cluster);
    const across = beside([stacked(of('teams'), CARD), shaped(of('sessions'), shape, GAP), stacked(of('resources'), CARD)].filter((block) => block.parts.length), BETWEEN);
    const singles = reading(here.loose).map(alone);
    const beneath = across.parts.length ? rows(singles, across.w, CARD) : shaped(singles, shape, CARD);
    return stacked([across, beneath].filter((block) => block.parts.length), GAP);
  };

  const whole = laid('');
  const all = [...windows, ...ranked, ...loose];
  const [x, y] = all.length ? [Math.min(...all.map((thing) => thing.x)), Math.min(...all.map((thing) => thing.y))] : [0, 0];
  const now = new Map(shifted(whole, x, y).map((part) => [part.id, part]));
  const moved = <T extends Thing>(each: T): T => { const to = now.get(each.id); return to ? { ...each, x: to.x, y: to.y } : each; };
  return {
    boxes: Object.fromEntries(windows.map((each) => { const { x: left, y: top, w, h } = now.get(each.id) ?? each; return [each.id, { x: left, y: top, w, h }]; })),
    groups: marks.groups.map((group) => { const to = now.get(group.id); return to ? { ...group, x: to.x, y: to.y, w: to.w, h: to.h } : group; }),
    notes: marks.notes.map(moved), widgets: marks.widgets.map(moved),
    extent: { x, y, w: whole.w, h: whole.h },
  };
}
