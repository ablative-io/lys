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

/**
 * What the agent's policy forces into the Refused list at a start: only its
 * hard rules, written as crates/lys-home/src/harness/rendering_permissions.rs
 * (lines 44 to 55) writes them. A rule that file refuses to write is left out.
 */
export function forcedBy(rules: PolicyRule[]): string[] {
  return rules.flatMap((rule) => {
    if (rule.authority !== 'hard') return [];
    if (rule.kind === 'tool' && rule.target === undefined) return [rule.tool];
    if (rule.kind === 'path_prefix' && rule.target !== undefined && (rule.tool === 'Read' || rule.tool === 'Edit') && !/[*?[\]()\\!]/.test(rule.target)) return [rule.tool + '(/' + rule.target + ')', rule.tool + '(/' + rule.target + '/**)'];
    if (rule.kind === 'host' && rule.target !== undefined && rule.tool === 'WebFetch') return ['WebFetch(domain:' + rule.target + ')'];
    return [];
  });
}
