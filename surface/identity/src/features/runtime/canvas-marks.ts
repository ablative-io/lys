/**
 * What a person draws on the canvas themselves: a box around some windows with a label, a note to write in, a line
 * between two things. None of it changes anything but the picture; it is theirs, kept with where their windows sit.
 */

/** Where a thing sits on the surface and how large it is, in the surface's own units. */
export interface Box { x: number; y: number; w: number; h: number }
/** A labelled box drawn around windows: "the people working on Iridium". */
export interface Group extends Box { id: string; label: string; colour?: string }
/** A note the person writes on the surface. */
export interface Note extends Box { id: string; text: string; colour?: string }
/** An edge of a thing: where its dot is, and where a line leaves it or meets it. */
export type Side = 'top' | 'right' | 'bottom' | 'left';
export const SIDES: Side[] = ['top', 'right', 'bottom', 'left'];
/**
 * A line the person drew between two things on the surface. It changes nothing but the picture. Dragged from a dot, it
 * leaves from that edge and meets the edge it was let go nearest; drawn by pressing two things, it takes the facing edges.
 */
export interface Link { id: string; from: string; to: string; from_side?: Side; to_side?: Side }
/**
 * A widget: one kind of what Lys holds about agents, put on the surface. The lines drawn to it feed it: it counts each
 * agent, and each box of agents, a line joins it to. With no line it counts the agents in the box it sits in, and every
 * agent when it sits in none.
 */
export interface Widget extends Box {
  id: string; kind: string;
  /** Which one of the kind's figures its small face shows, when the person chose one; the first that was reported otherwise. */
  shows?: string;
  /** The figures its medium face holds side by side, when the person chose them; the kind's own few otherwise. */
  faces?: string[];
  /** The face it shows beyond the small one, the pill: medium (a few figures), large (everything it holds), or its settings. `w` and `h` are its size in any of them. */
  view?: 'faces' | 'detail' | 'settings';
  /** How a figure that is a percent is drawn: as its number alone when absent, with a bar, or as a dial. */
  look?: 'bar' | 'dial';
  colour?: string;
  /** Locked, it is not moved, changed or taken away until it is unlocked. */
  locked?: boolean;
}
/**
 * The colours a person gives a box, a note or a widget: a few that sit with Lys's own, each by its name. A thing with no
 * colour, or one this page does not know, wears Lys's accent.
 */
export const COLOURS: Record<string, string> = { green: '#74b584', amber: '#fbbf24', red: '#f87171', blue: '#7aa7d9', violet: '#a58bd6', teal: '#6dbfb8', rose: '#d98aa5', grey: '#9a9aa3' };
/** The style that tints a thing its colour. */
export const tint = (colour: string | undefined): Record<string, string> => colour && COLOURS[colour] ? { '--tint': COLOURS[colour] } : {};
/** The colour after this one, going round: Lys's own, then each of the others. */
export function nextColour(colour: string | undefined): string | undefined {
  const names = Object.keys(COLOURS);
  return names[colour === undefined ? 0 : names.indexOf(colour) + 1 || names.length];
}
/**
 * A widget's views in the order its own button goes through them: small (the pill), medium (a few figures side by side),
 * large (everything it holds), its settings. A kind with no figures to choose among has no medium face.
 */
export function nextView(view: Widget['view'], faced: boolean): Widget['view'] {
  if (view === undefined) return faced ? 'faces' : 'detail';
  return view === 'faces' ? 'detail' : view === 'detail' ? 'settings' : undefined;
}
/** The size of a widget that is not opened out: a pill holding its symbol, whose it is, and its one figure. */
export const PILL: [number, number] = [248, 34];
/** A widget as it stands on the surface now: a pill, or its own size when it is opened out. */
export const standing = (widget: Widget): Widget => widget.view ? widget : { ...widget, w: PILL[0], h: PILL[1] };
/** The gap kept between a widget opened out and one it moved down. */
const CLEAR = 10;
/**
 * Every widget as it stands now. One opened out is as large as what it holds, so it moves the widgets it would cover
 * down, clear of it, and they move what they would cover in turn. Where the person put each one is kept: it stands
 * there again when the one above it is a pill.
 */
export function stood(widgets: Widget[]): Widget[] {
  const placed: { now: Widget; pushes: boolean }[] = [];
  for (const widget of [...widgets].sort((left, right) => left.y - right.y)) {
    let now = standing(widget);
    for (const above of [...placed].sort((left, right) => left.now.y - right.now.y)) {
      const beside = now.x < above.now.x + above.now.w && above.now.x < now.x + now.w;
      if (above.pushes && beside && now.y >= above.now.y && now.y < above.now.y + above.now.h + CLEAR) now = { ...now, y: above.now.y + above.now.h + CLEAR };
    }
    placed.push({ now, pushes: !!widget.view || now.y !== widget.y });
  }
  return widgets.map((widget) => placed.find((each) => each.now.id === widget.id)?.now ?? widget);
}
export interface Marks { groups: Group[]; notes: Note[]; links: Link[]; widgets: Widget[] }
/** Everything a person arranged: where each window sits, which terminals are open, and what they drew. */
export interface Arrangement extends Marks { boxes: Record<string, Box>; open: string[] }

export const NO_MARKS: Marks = { groups: [], notes: [], links: [], widgets: [] };
/** The size a note or a box is made at when it was not drawn out, and the smallest each is dragged to. */
export const NOTE: [number, number] = [260, 180];
export const GROUP: [number, number] = [520, 360];
export const SMALLEST_MARK: [number, number] = [140, 80];

const record = (value: unknown): value is Record<string, unknown> => !!value && typeof value === 'object' && !Array.isArray(value);
export const isBox = (value: unknown): value is Box => record(value) && ['x', 'y', 'w', 'h'].every((key) => Number.isFinite(value[key]));
const worded = (value: unknown, word: string): boolean => record(value) && typeof value.id === 'string' && typeof value[word] === 'string' && (value.colour === undefined || typeof value.colour === 'string');
const isGroup = (value: unknown): value is Group => isBox(value) && worded(value, 'label');
const isNote = (value: unknown): value is Note => isBox(value) && worded(value, 'text');
const named = (value: unknown): boolean => value === undefined || (Array.isArray(value) && value.every((each) => typeof each === 'string'));
const isWidget = (value: unknown): value is Widget => isBox(value) && worded(value, 'kind') && ((value as Widget).shows === undefined || typeof (value as Widget).shows === 'string') && named((value as Widget).faces) && [undefined, 'faces', 'detail', 'settings'].includes((value as Widget).view) && [undefined, 'bar', 'dial'].includes((value as Widget).look) && [undefined, true, false].includes((value as Widget).locked);
const sided = (value: unknown): boolean => value === undefined || SIDES.includes(value as Side);
const isLink = (value: unknown): value is Link => worded(value, 'from') && worded(value, 'to') && sided((value as Link).from_side) && sided((value as Link).to_side);
/** A list that was not kept at all is an empty one; a list holding anything else cannot be read. */
function listOf<T>(value: unknown, each: (item: unknown) => item is T): T[] | null {
  if (value === undefined) return [];
  return Array.isArray(value) && value.every(each) ? value : null;
}

/** An arrangement as it was kept, or null when what was kept cannot be read as one. */
export function readArrangement(value: unknown): Arrangement | null {
  if (!record(value)) return null;
  const { boxes, open } = value;
  if (!record(boxes) || !Object.values(boxes).every(isBox)) return null;
  if (!Array.isArray(open) || !open.every((id) => typeof id === 'string')) return null;
  const [groups, notes, links, widgets] = [listOf(value.groups, isGroup), listOf(value.notes, isNote), listOf(value.links, isLink), listOf(value.widgets, isWidget)];
  if (!groups || !notes || !links || !widgets) return null;
  return { boxes: boxes as Record<string, Box>, open: open as string[], groups, notes, links, widgets };
}

/** A name for a new mark that no other has. */
export function markId(kind: 'group' | 'note' | 'link' | 'widget'): string {
  const bytes = new Uint8Array(8);
  crypto.getRandomValues(bytes);
  return kind + ':' + [...bytes].map((byte) => byte.toString(16).padStart(2, '0')).join('');
}

/** Whether a thing sits in a box: its middle is inside. A box moved takes what sits in it along. */
export const within = (group: Box, box: Box): boolean => {
  const [x, y] = [box.x + box.w / 2, box.y + box.h / 2];
  return x >= group.x && x <= group.x + group.w && y >= group.y && y <= group.y + group.h;
};

/** Where a drawn line leaves one thing and meets the other: the facing edges, at their middles. */
export function linkEnds(from: Box, to: Box): [number, number, number, number] {
  const [fx, fy, tx, ty] = [from.x + from.w / 2, from.y + from.h / 2, to.x + to.w / 2, to.y + to.h / 2];
  if (Math.abs(tx - fx) >= Math.abs(ty - fy)) return tx >= fx ? [from.x + from.w, fy, to.x, ty] : [from.x, fy, to.x + to.w, ty];
  return ty >= fy ? [fx, from.y + from.h, tx, to.y] : [fx, from.y, tx, to.y + to.h];
}

/** The middle of one edge of a thing. */
export function sidePoint(box: Box, side: Side): [number, number] {
  if (side === 'top') return [box.x + box.w / 2, box.y];
  if (side === 'bottom') return [box.x + box.w / 2, box.y + box.h];
  return [side === 'left' ? box.x : box.x + box.w, box.y + box.h / 2];
}

/** The edge of a thing nearest a point: where a line let go there meets it. */
export function nearestSide(box: Box, [x, y]: [number, number]): Side {
  const far = (side: Side) => { const [sx, sy] = sidePoint(box, side); return (sx - x) ** 2 + (sy - y) ** 2; };
  return SIDES.reduce((best, side) => far(side) < far(best) ? side : best);
}

/** The edges two things face each other with, for a line that names none. */
export function facing(from: Box, to: Box): [Side, Side] {
  const [dx, dy] = [to.x + to.w / 2 - (from.x + from.w / 2), to.y + to.h / 2 - (from.y + from.h / 2)];
  if (Math.abs(dx) >= Math.abs(dy)) return dx >= 0 ? ['right', 'left'] : ['left', 'right'];
  return dy >= 0 ? ['bottom', 'top'] : ['top', 'bottom'];
}

const across = (side: Side): boolean => side === 'left' || side === 'right';

/** The most a line's turn is rounded by; a short leg is rounded by half its length. */
const TURN = 14;

/** The path through `points`, each turn rounded over. */
function rounded(points: [number, number][]): string {
  let d = `M ${points[0][0]} ${points[0][1]}`;
  for (let index = 1; index < points.length; index += 1) {
    const [[px, py], [x, y], next] = [points[index - 1], points[index], points[index + 1]];
    const [before, after] = [Math.hypot(x - px, y - py), next ? Math.hypot(next[0] - x, next[1] - y) : 0];
    const r = Math.min(TURN, before / 2, after / 2);
    if (!next || r === 0) { d += ` L ${x} ${y}`; continue; }
    d += ` L ${x - (x - px) / before * r} ${y - (y - py) / before * r} Q ${x} ${y} ${x + (next[0] - x) / after * r} ${y + (next[1] - y) / after * r}`;
  }
  return d;
}

/**
 * A line's way from one thing to another: it leaves the edge it was drawn from, runs straight, and turns to meet the
 * other's edge, each turn rounded over; never a diagonal. `middle` is a point on it, where its own small control sits.
 */
export function linkRoute(from: Box, to: Box, link: Pick<Link, 'from_side' | 'to_side'>): { d: string; middle: [number, number] } {
  const [facingFrom, facingTo] = facing(from, to);
  const [out, into] = [link.from_side ?? facingFrom, link.to_side ?? facingTo];
  const [[x, y], [ex, ey]] = [sidePoint(from, out), sidePoint(to, into)];
  if (across(out) && across(into)) { const turn = (x + ex) / 2; return { d: rounded([[x, y], [turn, y], [turn, ey], [ex, ey]]), middle: [turn, (y + ey) / 2] }; }
  if (!across(out) && !across(into)) { const turn = (y + ey) / 2; return { d: rounded([[x, y], [x, turn], [ex, turn], [ex, ey]]), middle: [(x + ex) / 2, turn] }; }
  return across(out) ? { d: rounded([[x, y], [ex, y], [ex, ey]]), middle: [ex, y] } : { d: rounded([[x, y], [x, ey], [ex, ey]]), middle: [x, ey] };
}
