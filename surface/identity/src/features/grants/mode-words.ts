/**
 * A grant's mode in words (ACCESS-001 R4): outright, by draft, or by two
 * approvals. The mode is the one the service answers; nothing here judges it.
 */
import type { GrantMode } from '../../generated/grants';

const WORDS: Record<GrantMode, string> = { outright: 'outright', by_draft: 'by draft', by_two: 'by two approvals' };

/** The modes an issuer may choose, in the order the form offers them. */
export const MODES: GrantMode[] = ['outright', 'by_draft', 'by_two'];

/** The mode in words; a mode the service named outside the three is said by its own name, never passed off as outright. */
export function modeWords(mode: string): string {
  return (WORDS as Record<string, string>)[mode] ?? mode;
}

/** What each mode says to the issuer choosing it. */
export const MODE_HINTS: Record<GrantMode, string> = {
  outright: 'The holder acts at once.',
  by_draft: 'Each act waits as a draft until one approver approves it.',
  by_two: 'Each act waits as a draft until two different approvers approve it.',
};
