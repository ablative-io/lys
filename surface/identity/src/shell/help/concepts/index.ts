import { ACCESS } from './access';
import { APPS } from './apps';
import { BUDGETS } from './budgets';
import { COMPUTERS } from './computers';
import { PEOPLE } from './people';
import { START } from './start';
import { TROUBLE } from './trouble';
import type { Concept, Seed, TrackId } from './types';

export type { Concept, Step, TrackId } from './types';

/** The tracks in the order the help index shows them, each with its heading. */
export const TRACKS: readonly (readonly [TrackId, string])[] = [
  ['start', 'Getting started'],
  ['people', 'People and agents'],
  ['access', 'Access and permissions'],
  ['computers', 'Computers'],
  ['apps', 'Apps'],
  ['budgets', 'Budgets'],
  ['trouble', 'Something isn’t working?'],
];

const tagged = (track: TrackId, seeds: Seed[]): Concept[] => seeds.map((seed) => ({ ...seed, track }));

/** Every concept, one place. */
export const CONCEPTS: readonly Concept[] = [
  ...tagged('start', START),
  ...tagged('people', PEOPLE),
  ...tagged('access', ACCESS),
  ...tagged('computers', COMPUTERS),
  ...tagged('apps', APPS),
  ...tagged('budgets', BUDGETS),
  ...tagged('trouble', TROUBLE),
];

export const conceptById = (id: string): Concept | undefined => CONCEPTS.find((concept) => concept.id === id);

/** The words a filter is matched against: the title, summary, explanation and steps. */
const words = (concept: Concept): string =>
  [concept.title, concept.summary, ...concept.body, ...concept.steps.map((step) => step.text)].join(' ').toLowerCase();

/**
 * The index, by track, narrowed to the concepts holding every word typed. A track with nothing left is left out,
 * so an empty answer is an empty list, said by the caller.
 */
export function conceptGroups(query: string): { track: TrackId; label: string; concepts: Concept[] }[] {
  const terms = query.toLowerCase().split(/\s+/).filter(Boolean);
  return TRACKS.map(([track, label]) => ({
    track,
    label,
    concepts: CONCEPTS.filter((concept) => concept.track === track && terms.every((term) => words(concept).includes(term))),
  })).filter((group) => group.concepts.length > 0);
}
