import { kindOf } from '../../generated';
import type { IdentityKind, LifecycleState, PeopleView, PersonSummary } from '../../generated';

/** One row of the directory: a person, or an agent with the person it answers to. */
export interface Entry {
  id: string;
  display_name: string;
  state: LifecycleState;
  kind: IdentityKind;
  /** The role's title, when the service names one; the people view names none yet. */
  role: string | null;
  person: PersonSummary | null;
}

/** People first, then every agent, as the mock-up lists them. */
export function entries(view: PeopleView): Entry[] {
  const people: Entry[] = view.people.map((p) => ({ id: p.id, display_name: p.display_name, state: p.state, kind: 'person', role: null, person: null }));
  const agents: Entry[] = view.people.flatMap((p) =>
    p.agents.map((a) => ({
      id: a.id,
      display_name: a.display_name,
      state: a.state,
      kind: 'agent' as const,
      role: null,
      person: { id: p.id, display_name: p.display_name, state: p.state },
    })),
  );
  return [...people, ...agents];
}

/** The file number of an identity: the kind's letter (A an agent, S a service account, whose id begins `op-`, P a person) and the head of the enduring id. */
export const fileNo = (id: string): string => {
  const hex = id.slice(id.indexOf('-') + 1);
  return (kindOf(id) === 'agent' ? 'A/' : id.startsWith('op-') ? 'S/' : 'P/') + hex.slice(0, 8);
};

/** The first word of a name, exactly as the mock-up writes `x.name.split(' ')[0]`. */
export const firstName = (name: string): string => name.split(' ')[0];

/** How a sentence names someone: a person by their first word, an agent by its whole name, since "Walk" is not "Walk Helper". */
export const calledBy = (id: string, name: string): string => (kindOf(id) === 'agent' ? name : firstName(name));

/** An agent still standing whose person is not active needs a new person (conformance 3.1). */
export const needsNewPerson = (x: Entry): boolean =>
  x.kind === 'agent' && x.state !== 'retired' && x.person !== null && x.person.state !== 'active';
