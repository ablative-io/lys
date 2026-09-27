import { kindOf } from '../../generated';
import type { IdentityKind, LifecycleState, PeopleView, PersonSummary } from '../../generated';

/** One row of the directory: a person, or an agent with the person it answers to. */
export interface Entry {
  id: string;
  display_name: string;
  state: LifecycleState;
  kind: IdentityKind;
  person: PersonSummary | null;
}

/** People first, then every agent, as the mock-up lists them. */
export function entries(view: PeopleView): Entry[] {
  const people: Entry[] = view.people.map((p) => ({ id: p.id, display_name: p.display_name, state: p.state, kind: 'person', person: null }));
  const agents: Entry[] = view.people.flatMap((p) =>
    p.agents.map((a) => ({
      id: a.id,
      display_name: a.display_name,
      state: a.state,
      kind: 'agent' as const,
      person: { id: p.id, display_name: p.display_name, state: p.state },
    })),
  );
  return [...people, ...agents];
}

/** The file number: the kind's letter and the head of the enduring id. */
export const fileNo = (id: string): string => {
  const hex = id.slice(id.indexOf('-') + 1);
  return (kindOf(id) === 'agent' ? 'A/' : 'P/') + hex.slice(0, 8);
};

/** The first word of a name, exactly as the mock-up writes `x.name.split(' ')[0]`. */
export const firstName = (name: string): string => name.split(' ')[0];

/** An agent still standing whose person is retired needs a new person (conformance 3.1). */
export const needsNewPerson = (x: Entry): boolean =>
  x.kind === 'agent' && x.state !== 'retired' && x.person?.state === 'retired';
