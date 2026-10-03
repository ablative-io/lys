/**
 * The rules an agent's settings carry, built and read in plain words.
 *
 * A person never writes a rule's grammar. They choose a kind and give the
 * one thing that kind needs; `ruleFor` writes the rule as Claude Code reads
 * it (docs/harness/reference/claude-code/permissions.md):
 *
 * - a folder is written with two leading slashes, `Read(//srv/site/**)`,
 *   because a path with one leading slash in the file Lys passes means
 *   "beside that file", not the top of the disk (lines 353-369);
 * - a command is written `Bash(git push *)`, which matches the command by
 *   itself and with anything after it (lines 150-191);
 * - a website is written `WebFetch(domain:example.com)` (lines 471-477);
 * - a whole tool is its bare name (lines 99-109).
 *
 * The service accepts a rule only as `Tool` or `Tool(specifier)` with no
 * round brackets inside the specifier (crates/lys-home/src/harness/
 * rendering_permissions.rs, `expressible`), so a target holding one is
 * refused here, in words, before anything is sent.
 */
import type { Permissions } from './Provisioning';

export type RuleKind = 'read-folder' | 'change-folder' | 'command' | 'website' | 'tool';

/** The five kinds, in the order they are offered, each as a person would say it. */
export const RULE_KINDS: readonly (readonly [RuleKind, string])[] = [
  ['read-folder', 'Read files under a folder'],
  ['change-folder', 'Change files under a folder'],
  ['command', 'Run a command that starts with…'],
  ['website', 'Reach a website'],
  ['tool', 'A whole tool, by its name'],
];

export type Built = { rule: string } | { problem: string };

const plain = (text: string) => ![...text].some((c) => c < ' ' || c === '\u007f');

/** The rule for a kind and its one target, or the sentence that says why it cannot be written. */
export function ruleFor(kind: RuleKind, given: string): Built {
  const target = given.trim();
  if (!target) return { problem: kind === 'command' ? 'Type the words the command starts with.' : kind === 'website' ? 'Type the website’s name.' : kind === 'tool' ? 'Type the tool’s name.' : 'Choose a folder.' };
  if (!plain(target)) return { problem: 'That holds a character that cannot be written in a rule.' };
  if (/[()]/.test(target)) return { problem: 'A rule cannot hold a round bracket. Leave the brackets out.' };
  if (kind === 'read-folder' || kind === 'change-folder') {
    if (!target.startsWith('/')) return { problem: 'Choose a folder from the computer’s own folders.' };
    if (/[*?[\]!\\]/.test(target)) return { problem: 'That folder’s name holds a character a rule reads as a pattern (* ? [ ] ! or \\), so a rule for it would match other folders too.' };
    const folder = target === '/' ? '' : target.replace(/\/+$/, '');
    return { rule: (kind === 'read-folder' ? 'Read' : 'Edit') + '(/' + folder + '/**)' };
  }
  if (kind === 'command') return { rule: 'Bash(' + target.replace(/\s+\*$/, '') + ' *)' };
  if (kind === 'website') {
    const host = target.toLowerCase().replace(/\.$/, '');
    if (!/^(\*\.)?[a-z0-9-]+(\.[a-z0-9-]+)+$/.test(host)) return { problem: 'Type the website’s name alone, like example.com, with no https:// and nothing after the name.' };
    return { rule: 'WebFetch(domain:' + host + ')' };
  }
  if (!/^[A-Za-z][A-Za-z0-9_-]*$/.test(target)) return { problem: 'A tool’s name is letters, digits, _ and -, starting with a letter, exactly as the program names it.' };
  return { rule: target };
}

const WHOLE: Record<string, string> = {
  Bash: 'Run any command',
  Read: 'Read any file',
  Edit: 'Change any file',
  Write: 'Write any file',
  WebFetch: 'Fetch any web page',
  WebSearch: 'Search the web',
};

/** A rule in plain words. A rule this screen did not write and cannot read is shown as itself. */
export function wordsFor(rule: string): string {
  const open = rule.indexOf('(');
  if (open < 0) return WHOLE[rule] ?? (rule.startsWith('mcp__') ? 'Use the connected tools named ' + rule.slice(5) : 'Use the tool ' + rule);
  if (!rule.endsWith(')')) return 'The rule ' + rule;
  const tool = rule.slice(0, open);
  const inside = rule.slice(open + 1, -1);
  if ((tool === 'Read' || tool === 'Edit') && inside.startsWith('//') && inside.endsWith('/**')) {
    return (tool === 'Read' ? 'Read files under ' : 'Change files under ') + (inside.slice(1, -3) || '/');
  }
  if (tool === 'Bash' && inside.endsWith(' *')) return 'Run commands that start with ' + inside.slice(0, -2);
  if (tool === 'Bash' && !inside.includes('*')) return 'Run the command ' + inside;
  if (tool === 'WebFetch' && inside.startsWith('domain:')) return 'Reach ' + inside.slice(7);
  return 'The rule ' + rule;
}

const count = (n: number, one: string, many: string) => n + ' ' + (n === 1 ? one : many);

/** One line that sums the rules without listing them. */
export function summary(permissions: Permissions | null | undefined): string {
  const refused = permissions?.deny?.length ?? 0;
  const asks = permissions?.ask?.length ?? 0;
  const without = permissions?.allow?.length ?? 0;
  const folders = permissions?.additional_directories?.length ?? 0;
  const total = refused + asks + without;
  const parts = [refused ? refused + ' refused' : '', asks ? asks + (asks === 1 ? ' asks first' : ' ask first') : '', without ? without + ' without asking' : ''].filter(Boolean);
  const rules = total ? count(total, 'rule', 'rules') + ': ' + parts.join(', ') + '.' : 'No extra rules.';
  return folders ? rules + ' ' + count(folders, 'extra folder', 'extra folders') + '.' : rules;
}
