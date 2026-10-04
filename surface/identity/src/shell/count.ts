/** Count sentences, singular for one: "1 computer", "2 computers", "1 person active". */

/** A whole number as Australian English writes it: 1,500. */
export const count = (n: number): string => n.toLocaleString('en-AU');

/** The words that do not lose an s for one. */
const IRREGULAR: Record<string, string> = {
  people: 'person',
  'people and agents': 'person or agent',
  'running sessions': 'running session',
  'grants to review': 'grant to review',
};

/** One of a plural noun: "computers" → "computer", "people" → "person", "people active" → "person active". */
export function singular(noun: string): string {
  if (IRREGULAR[noun]) return IRREGULAR[noun];
  const [first, ...rest] = noun.split(' ');
  const one = IRREGULAR[first] ?? (first.endsWith('ies') ? first.slice(0, -3) + 'y' : first.endsWith('s') && !first.endsWith('ss') ? first.slice(0, -1) : first);
  return [one, ...rest].join(' ');
}

/** A count and its noun, singular for one: counted(1, 'computers') → "1 computer". */
export const counted = (n: number, plural: string, one: string = singular(plural)): string => count(n) + ' ' + (n === 1 ? one : plural);
