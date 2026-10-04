/**
 * A box round each team, drawn for the person in one press. Every team with an agent running gets a box named for it (a
 * retired team's membership is a record, not a team, and gets none),
 * holding the team's own card and its agents' windows; a team's own teams are boxes inside its box, so the hierarchy is
 * seen. A team with nothing running is drawn only when one of its own teams is. Teams that name each other as parent are neither inside the other. An agent in several teams stands in one
 * box: the team furthest down the hierarchy, and of two as far down the one first by name. The boxes are ordinary
 * boxes after: moved, renamed, coloured and removed like any the person drew. Pressed again, the team boxes are drawn
 * afresh in place of the ones drawn before. What is made here is only placed roughly, clear of everything else; Tidy
 * lays it out.
 */
import { SMALLEST_MARK } from './canvas-marks';
import type { Box, Group, Marks } from './canvas-marks';
import type { SessionGraph } from './session-graph';

/** The name a team's box is kept under, so the next press replaces it. */
const BOX = 'group:team:';
const [BAR, EDGE, GAP, APART] = [40, 20, 20, 200];

interface Placed { w: number; h: number; windows: Record<string, Box>; groups: Group[] }

/** The canvas with a box round each team: where each window now stands and the marks with the boxes in them; null when no team has an agent running. */
export function teamBoxed(graph: Pick<SessionGraph, 'nodes' | 'edges' | 'teams'>, boxes: Record<string, Box>, marks: Marks): { boxes: Record<string, Box>; marks: Marks } | null {
  const known = graph.teams ?? {};
  const name = (team: string) => known[team]?.name ?? graph.nodes.find((node) => node.id === 'team:' + team)?.title ?? 'Team';
  /** The team a team is one of: none when it names none, names one that is not known, or the naming comes back round to itself. */
  const above = (team: string): string | null => {
    const parent = known[team]?.parent;
    if (!parent || !known[parent]) return null;
    const seen = [team];
    for (let at: string | null | undefined = parent; at && known[at]; at = known[at].parent) { if (seen.includes(at)) return null; seen.push(at); }
    return parent;
  };
  /** A team's line of parents, itself first. */
  const line = (team: string): string[] => { const up = above(team); return up ? [team, ...line(up)] : [team]; };
  // Each window's one team.
  const members = new Map<string, string[]>();
  const teamsOf = new Map<string, string[]>();
  for (const edge of graph.edges) if (edge.kind === 'membership' && edge.stands && edge.to in boxes) teamsOf.set(edge.to, [...teamsOf.get(edge.to) ?? [], edge.from.slice('team:'.length)]);
  for (const [window, teams] of teamsOf) {
    const team = [...teams].sort((left, right) => line(right).length - line(left).length || name(left).localeCompare(name(right)) || left.localeCompare(right))[0];
    members.set(team, [...members.get(team) ?? [], window]);
  }
  if (!members.size) return null;
  // Every team drawn: one with a window, and each team above it.
  const drawn = new Set([...members.keys()].flatMap(line));
  const under = (parent: string | null) => [...drawn].filter((team) => above(team) === parent).sort((left, right) => name(left).localeCompare(name(right)) || left.localeCompare(right));
  const before = new Map(marks.groups.map((group) => [group.id, group]));

  /** One team's box at (`x`, `y`): its card, its windows, then its own teams' boxes, one under another. */
  const placed = (team: string, x: number, y: number): Placed => {
    const windows: Record<string, Box> = {};
    const groups: Group[] = [];
    let [down, wide] = [y + BAR, 0];
    for (const id of ['team:' + team, ...members.get(team) ?? []]) {
      const box = boxes[id];
      if (!box) continue;
      windows[id] = { ...box, x: x + EDGE, y: down };
      [down, wide] = [down + box.h + GAP, Math.max(wide, box.w)];
    }
    for (const child of under(team)) {
      const inner = placed(child, x + EDGE, down);
      Object.assign(windows, inner.windows);
      groups.push(...inner.groups);
      [down, wide] = [down + inner.h + GAP, Math.max(wide, inner.w)];
    }
    const [w, h] = [Math.max(SMALLEST_MARK[0], wide + 2 * EDGE), down - GAP + EDGE - y];
    return { w, h, windows, groups: [{ id: BOX + team, label: name(team), x, y, w, h, ...(before.get(BOX + team)?.colour ? { colour: before.get(BOX + team)!.colour } : {}) }, ...groups] };
  };

  // The boxes stand side by side, clear to the right of everything on the canvas, so nothing else lies in one.
  const kept = marks.groups.filter((group) => !group.id.startsWith(BOX));
  const all: Box[] = [...Object.values(boxes), ...kept, ...marks.notes, ...marks.widgets];
  let x = Math.max(...all.map((each) => each.x + each.w)) + APART;
  const y = Math.min(...all.map((each) => each.y));
  const next = { boxes: { ...boxes }, groups: [...kept] };
  for (const team of under(null)) {
    const one = placed(team, x, y);
    Object.assign(next.boxes, one.windows);
    next.groups.push(...one.groups);
    x += one.w + APART;
  }
  return { boxes: next.boxes, marks: { ...marks, groups: next.groups } };
}
