/** The plain names of the modes, the three rule lists, and what an agent's policy forces into the Refused list. The rules themselves are built and read in permission-rules.ts. */
import type { Rule as PolicyRule } from '../file/policyContract';

export type RuleList = 'allow' | 'ask' | 'deny';
export const LISTS: { id: RuleList; name: string }[] = [
  { id: 'allow', name: 'Without asking' }, { id: 'ask', name: 'Asks first' }, { id: 'deny', name: 'Refused' },
];

/** The plain name of each mode the fact tables know, and whether it removes every check. A mode not here is shown by its own id. */
export const MODES: Record<string, { name: string; warn?: true }> = {
  'workspace-only': { name: 'Kept to its folder' },
  default: { name: 'Asks before it acts' },
  acceptEdits: { name: 'Changes files in its folder without asking' },
  plan: { name: 'Looks and plans, changes nothing' },
  auto: { name: 'Decides for itself, checked by Claude Code' },
  dontAsk: { name: 'Only what is listed, never asks' },
  bypassPermissions: { name: 'No checks at all', warn: true },
  'read-only': { name: 'Reads, changes nothing' },
  'workspace-write': { name: 'Works in its folder' },
  'danger-full-access': { name: 'No limits', warn: true },
};
/** The modes shown before "More ways it can work" is opened, in this order; the rest follow in the catalogue's order. */
export const FIRST = ['workspace-only', 'default', 'acceptEdits', 'read-only', 'workspace-write', 'danger-full-access'];

/** The first sentence of a meaning, and what follows it. */
export function firstSentence(meaning: string): { first: string; rest: string } {
  const end = meaning.search(/[.!?](\s|$)/);
  return end < 0 ? { first: meaning, rest: '' } : { first: meaning.slice(0, end + 1), rest: meaning.slice(end + 1).trim() };
}

/** What an agent's policy does to a start: the rules it writes into the Refused list, the rules the start cannot write at all, and how many hard rules there are. */
export interface Forced { written: string[]; unwritable: string[]; hard: number }

/** A tool name the settings file reads: a letter first, then letters, digits, `_` or `-` (crates/lys-home/src/harness/rendering_permissions.rs, `expressible`, lines 20 to 25). */
const named = (tool: string) => /^[A-Za-z][A-Za-z0-9_-]*$/.test(tool);

/**
 * Only the policy's hard rules reach a start, written as
 * crates/lys-home/src/harness/rendering_permissions.rs (`denied`, lines 41 to
 * 73) writes them. A hard rule that file refuses makes the start itself
 * fail, so it is returned by its id and never left out in silence.
 */
export function forcedBy(rules: PolicyRule[]): Forced {
  const forced: Forced = { written: [], unwritable: [], hard: 0 };
  for (const rule of rules) {
    if (rule.authority !== 'hard') continue;
    forced.hard += 1;
    const target = rule.target;
    if (!named(rule.tool)) forced.unwritable.push(rule.id);
    else if (rule.kind === 'tool' && target === undefined) forced.written.push(rule.tool);
    else if (rule.kind === 'path_prefix' && target !== undefined && (rule.tool === 'Read' || rule.tool === 'Edit') && !/[*?[\]()\\!]/.test(target)) forced.written.push(rule.tool + '(/' + target + ')', rule.tool + '(/' + target + '/**)');
    else if (rule.kind === 'host' && target !== undefined && rule.tool === 'WebFetch' && !/[()]/.test(target)) forced.written.push('WebFetch(domain:' + target + ')');
    else forced.unwritable.push(rule.id);
  }
  return forced;
}
