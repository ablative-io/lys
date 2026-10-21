/**
 * The help registry's shape: every explainable
 * thing is a concept with a stable dotted id, a one-sentence summary, a plain explanation, the steps that do it
 * and the concepts worth reading next. The id is what the address carries (`?topic=`), so renaming one breaks a
 * link and is done deliberately.
 */
export type TrackId = 'start' | 'people' | 'access' | 'computers' | 'apps' | 'budgets' | 'trouble';

export interface Concept {
  id: string;
  track: TrackId;
  /** The concept's name, as a reader would say it out loud. */
  title: string;
  /** Exactly one sentence: what it is. Shown in the index. */
  summary: string;
  /** Short paragraphs, plain words; one <p> each. */
  body: readonly string[];
  /** What to do, in order; each step may name the screen that does it. Empty when there is nothing to do. */
  steps: readonly Step[];
  /** Other concepts worth reading next; every id must resolve (registry test). */
  related: readonly string[];
}

export interface Step {
  text: string;
  /** The screen that does this step, as a hash address (`#/network`). */
  href?: string;
  /** What the link says. */
  link?: string;
}

/** A concept as written in a track's table: the track is the table's own. */
export type Seed = Omit<Concept, 'track'>;
