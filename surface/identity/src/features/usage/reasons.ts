/** Why a unit has no figure, said once in plain words with the holder's name: never an identifier, never the same sentence twice. */
import type { Measure } from './contract';

/** The holder the page is about: its identifier, matched in the service's words, and the name a person reads instead. */
export type Named = { id: string; name: string };

const IDENTIFIER = /\b(?:agent|person|team|grant|op)-[0-9A-Za-z]+\b/g;

/** One sentence of the service's, with any known identifier replaced by a name and any other by plain words. */
function sentence(part: string, holder: Named): string {
  const who = (id: string) => (id === holder.id ? holder.name : 'another agent');
  const live = /^dollars have not been reported for live session \S+ of agent (\S+)$/.exec(part);
  if (live) return 'No dollar spend has been reported for a running session of ' + who(live[1]) + '.';
  const dollars = /^dollars have not been reported for agent (\S+)$/.exec(part);
  if (dollars) return 'No dollar spend has been reported for ' + who(dollars[1]) + '.';
  const plan = /^plan window unreported for agent (\S+)$/.exec(part);
  if (plan) return 'No plan window has been reported for ' + who(plan[1]) + '.';
  const plain = part.replace(/\b(?:agent|person|team) ((?:agent|person|team)-[0-9A-Za-z]+)\b/g, '$1').replace(IDENTIFIER, (id) => who(id)).trim();
  const capital = plain.charAt(0).toUpperCase() + plain.slice(1);
  return /[.!?]$/.test(capital) ? capital : capital + '.';
}

/** One reason from the service as plain sentences, each once. */
export function plain(reason: string, holder: Named): string[] {
  return [...new Set(reason.split(';').map((part) => part.trim()).filter(Boolean).map((part) => sentence(part, holder)))];
}

/** Every unit's reason as sentences, each said once across the whole list and never one already said elsewhere on the page. */
export function reasons(unavailable: { unit: Measure; reason: string }[], holder: Named, already: Iterable<string> = []): string[] {
  const shown = new Set(already);
  const said = new Set<string>();
  for (const each of unavailable) {
    for (const words of plain(each.reason, holder)) if (!shown.has(words)) said.add(words);
  }
  return [...said];
}
