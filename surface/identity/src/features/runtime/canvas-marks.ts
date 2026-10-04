/**
 * What a person draws on the canvas themselves: a box around some windows with a label, a note to write in, a line
 * between two things. None of it changes anything but the picture; it is theirs, kept with where their windows sit.
 */

/** Where a thing sits on the surface and how large it is, in the surface's own units. */
export interface Box { x: number; y: number; w: number; h: number }
/** A labelled box drawn around windows: "the people working on Iridium". */
export interface Group extends Box { id: string; label: string }
/** A note the person writes on the surface. */
export interface Note extends Box { id: string; text: string }
/** A line the person drew between two things on the surface. It says nothing to the service. */
export interface Link { id: string; from: string; to: string }
export interface Marks { groups: Group[]; notes: Note[]; links: Link[] }
/** Everything a person arranged: where each window sits, which terminals are open, and what they drew. */
export interface Arrangement extends Marks { boxes: Record<string, Box>; open: string[] }

export const NO_MARKS: Marks = { groups: [], notes: [], links: [] };
/** The size a note or a box is made at when it was not drawn out, and the smallest each is dragged to. */
export const NOTE: [number, number] = [260, 180];
export const GROUP: [number, number] = [520, 360];
export const SMALLEST_MARK: [number, number] = [140, 80];

const record = (value: unknown): value is Record<string, unknown> => !!value && typeof value === 'object' && !Array.isArray(value);
export const isBox = (value: unknown): value is Box => record(value) && ['x', 'y', 'w', 'h'].every((key) => Number.isFinite(value[key]));
const worded = (value: unknown, word: string): boolean => record(value) && typeof value.id === 'string' && typeof value[word] === 'string';
const isGroup = (value: unknown): value is Group => isBox(value) && worded(value, 'label');
const isNote = (value: unknown): value is Note => isBox(value) && worded(value, 'text');
const isLink = (value: unknown): value is Link => worded(value, 'from') && worded(value, 'to');
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
  const [groups, notes, links] = [listOf(value.groups, isGroup), listOf(value.notes, isNote), listOf(value.links, isLink)];
  if (!groups || !notes || !links) return null;
  return { boxes: boxes as Record<string, Box>, open: open as string[], groups, notes, links };
}

/** A name for a new mark that no other has. */
export function markId(kind: 'group' | 'note' | 'link'): string {
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
