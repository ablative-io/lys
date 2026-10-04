/**
 * Widgets on the canvas: what Lys holds about agents, placed beside their windows. The lines do the feeding: a widget
 * counts each agent whose window a line joins it to, and the agents in each box a line joins it to. A widget no line
 * reaches counts the agents in the box it sits in, and every agent when it sits in none. Drawing a line, taking one
 * away, or moving a window into a box changes what a widget counts; nothing else has to be set.
 */
import { counted } from '../../shell/count';
import { refusedPart } from '../dashboard/contract';
import type { DashboardAgent } from '../dashboard/contract';
import type { AccountWindows, Length, Measure, Used } from '../usage/contract';
import { within } from './canvas-marks';
import type { Box, Group, Link, Widget } from './canvas-marks';

/** How a widget names the one agent it is of: by the agent, so it holds when the agent starts again. */
export const AGENT = 'agent:';

/** A kind of widget: its name, the size it opens out to, and its symbol as the path of a 24 by 24 line drawing. */
export interface Kind { label: string; size: [number, number]; symbol: string; /** The single figures a widget of this kind can be set to show, each by its name. */ choices?: Record<string, string> }
/** Every kind of widget, in the order the bar offers them. */
export const KINDS: Record<string, Kind> = {
  usage: { label: 'Usage', size: [340, 250], symbol: 'M4 17a8 8 0 1 1 16 0M12 17l4.5-6',
    choices: { 'context_percent/': 'Context', 'tokens/day': 'Tokens today', 'tokens/week': 'Tokens this week', 'dollars/day': 'Dollars today', 'dollars/week': 'Dollars this week', 'running_ms/day': 'Running today', 'window/300': '5-hour window', 'window/10080': '7-day window' } },
  budget: { label: 'Budget', size: [340, 170], symbol: 'M12 3v18M16.5 7.5C15.8 6.2 14.2 5.5 12 5.5c-2.6 0-4.5 1.2-4.5 3.2S9.300 11.5 12 12s4.500 1.300 4.500 3.300-1.900 3.200-4.500 3.200c-2.200 0-3.800-.700-4.500-2' },
  goals: { label: 'Goals', size: [340, 190], symbol: 'M6 21V4M6 4.500h11l-2.500 4 2.500 4H6' },
  requests: { label: 'Requests', size: [360, 190], symbol: 'M12 3a9 9 0 1 0 0 18 9 9 0 0 0 0-18zM9.600 9.500a2.500 2.500 0 1 1 3.900 2c-.900.600-1.500 1.200-1.500 2.300M12 17.200v.100' },
  drafts: { label: 'Drafts', size: [360, 190], symbol: 'M7 3h7l4 4v14H7zM14 3v4h4M10 13h5M10 17h5' },
};

/** A window on the surface that is an agent's: the name it has on the surface now, its agent, and where it stands. */
export interface AgentWindow { id: string; agent: string; box: Box }
/** Whose a widget is: the agents it counts, and that said in words. */
export interface Scope { rows: DashboardAgent[]; whose: string; everyone: boolean }

/** The agents in a box: those whose windows sit in it. */
const inBox = (group: Group, windows: AgentWindow[]): string[] => windows.filter((each) => within(group, each.box)).map((each) => each.agent);

/**
 * The agents a widget counts, and whose it is in words. Lines come first: every agent and every box a line joins the
 * widget to. A line's end kept under an agent's name is that agent's window. With no such line, the smallest box the
 * widget sits in; with none, every agent.
 */
export function scopeOf(widget: Widget, links: Link[], groups: Group[], windows: AgentWindow[], rows: DashboardAgent[]): Scope {
  const ends = links.flatMap((link) => link.from === widget.id ? [link.to] : link.to === widget.id ? [link.from] : []);
  const fedBy = ends.flatMap((end) => windows.filter((each) => each.id === end || AGENT + each.agent === end).map((each) => each.agent));
  const boxes = ends.flatMap((end) => groups.filter((group) => group.id === end));
  const fed = new Set([...fedBy, ...boxes.flatMap((group) => inBox(group, windows))]);
  if (fedBy.length || boxes.length) {
    const counted_rows = rows.filter((row) => fed.has(row.agent.id));
    const whose = !fedBy.length && boxes.length === 1 ? (boxes[0].label || 'This box') + ': ' + counted(fed.size, 'agents')
      : counted_rows.length === 1 && fed.size === 1 ? counted_rows[0].agent.display_name
      : fed.size <= 3 && counted_rows.length === fed.size ? counted_rows.map((row) => row.agent.display_name).join(', ') : counted(fed.size, 'agents');
    return { rows: counted_rows, whose, everyone: false };
  }
  const holding = groups.filter((group) => within(group, widget)).sort((left, right) => left.w * left.h - right.w * right.h)[0];
  if (!holding) return { rows, whose: 'Every agent', everyone: true };
  const inside = new Set(inBox(holding, windows));
  return { rows: rows.filter((row) => inside.has(row.agent.id)), whose: (holding.label || 'This box') + ': ' + counted(inside.size, 'agents'), everyone: false };
}

/** One figure over several agents: how many reported it, and why the others did not. */
export interface Summed { unit: Measure; period: Length | null; figure: number | null; reported: number; missing: string[] }

/** Levels are not amounts: context and an account's percent are shown at their highest, never added. */
const LEVEL: ReadonlySet<Measure> = new Set<Measure>(['context_percent', 'plan_percent']);

/**
 * The agents' figures as one line each. Tokens, running time and dollars are added over the agents that reported them;
 * context and account percent show the highest. An agent that reported nothing is counted as missing with its reason,
 * never as zero.
 */
export function summed(rows: DashboardAgent[]): Summed[] {
  const lines = new Map<string, Summed>();
  for (const row of rows) {
    if (refusedPart(row.usage)) continue;
    for (const used of (row.usage.figures ?? []) as Used[]) {
      const key = used.unit + '/' + (used.period ?? '');
      const line = lines.get(key) ?? { unit: used.unit, period: used.period, figure: null, reported: 0, missing: [] };
      if (used.figure === null) line.missing.push(row.agent.display_name + ': ' + used.unavailable);
      else {
        line.figure = line.figure === null ? used.figure : LEVEL.has(used.unit) ? Math.max(line.figure, used.figure) : line.figure + used.figure;
        line.reported += 1;
      }
      lines.set(key, line);
    }
  }
  return [...lines.values()];
}

/** An account's windows as last reported by any of the agents: the latest report of each account. */
export function accountsOf(rows: DashboardAgent[]) {
  const latest = new Map<string, AccountWindows>();
  for (const row of rows) {
    if (refusedPart(row.usage)) continue;
    for (const account of row.usage.accounts ?? []) {
      const held = latest.get(account.account);
      if (!held || held.at_ms < account.at_ms) latest.set(account.account, account);
    }
  }
  return [...latest.values()].sort((left, right) => left.account.localeCompare(right.account));
}
