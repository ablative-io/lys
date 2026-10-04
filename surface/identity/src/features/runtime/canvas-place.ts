/** Where each window first sits on the canvas, how large it is open and closed, and the service's own lines between windows. */
import { AGENT } from './canvas-widgets';
import type { Box } from './canvas-marks';
import type { SessionGraph } from './session-graph';

const BAR = 34;
const CARD: [number, number] = [220, 64];
/** A terminal window's size closed, when it is only its bar, and when it is first opened. */
export const CLOSED: [number, number] = [440, BAR];
export const OPENED: [number, number] = [760, 480];
/** The smallest a terminal window is dragged to: its bar still shows the agent's name beside state, Stop and close, and a prompt can still be read. */
export const SMALLEST: [number, number] = [560, 180];

/** A first place for every node that has none: teams down the left, agents in the middle two across, resources on the right. */
export function placed(graph: SessionGraph, have: Record<string, Box>, open: ReadonlySet<string>): Record<string, Box> {
  const boxes: Record<string, Box> = {};
  const sessions = graph.nodes.filter((node) => node.column === 'sessions');
  const across = Math.min(2, Math.max(1, sessions.length));
  const middle = CARD[0] + 60;
  const right = middle + across * (OPENED[0] + 40) + 20;
  const count = { teams: 0, sessions: 0, resources: 0 };
  for (const node of graph.nodes) {
    const index = count[node.column]++;
    // A place kept under the agent's name is the place of whichever session that agent has now; such a window stands closed until it is opened.
    const agent = node.session?.agent;
    const named = agent ? have[AGENT + agent] : undefined;
    const was = have[node.id] ?? (named && !open.has(node.id) ? { ...named, w: CLOSED[0], h: CLOSED[1] } : named);
    if (was) { boxes[node.id] = was; continue; }
    if (node.column === 'sessions') {
      const [w, h] = open.has(node.id) ? OPENED : CLOSED;
      boxes[node.id] = { x: middle + (index % across) * (OPENED[0] + 40), y: Math.floor(index / across) * (OPENED[1] + 40), w, h };
    } else boxes[node.id] = { x: node.column === 'teams' ? 0 : right, y: index * (CARD[1] + 20), w: CARD[0], h: CARD[1] };
  }
  return boxes;
}

/** A connection, drawn from the middle of one window's right edge to the middle of the other's left edge. */
export function lineBetween(from: Box, to: Box): string {
  const [x, y, endX, endY] = [from.x + from.w, from.y + from.h / 2, to.x, to.y + to.h / 2];
  const middle = (x + endX) / 2;
  return `M ${x} ${y} C ${middle} ${y}, ${middle} ${endY}, ${endX} ${endY}`;
}
