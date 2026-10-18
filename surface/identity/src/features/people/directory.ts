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
  /**
   * The agent this agent reports to, when it is an agent rather than its person. The person stays in `person`: an agent
   * always has a person who answers for it, even when it reports to another agent.
   */
  reportsTo?: { id: string; display_name: string | null } | null;
}

/** The `reports_to` edge the people view serves for an agent (`AgentSummary.reports_to` on the server), when it is there. */
function reportingEdge(agent: unknown): { id: string; kind: string } | null {
  const edge = agent && typeof agent === 'object' ? (agent as { reports_to?: unknown }).reports_to : undefined;
  return edge && typeof edge === 'object' && typeof (edge as { id?: unknown }).id === 'string' && typeof (edge as { kind?: unknown }).kind === 'string'
    ? edge as { id: string; kind: string } : null;
}

/** People first, then every agent, as the mock-up lists them. */
export function entries(view: PeopleView): Entry[] {
  const people: Entry[] = view.people.map((p) => ({ id: p.id, display_name: p.display_name, state: p.state, kind: 'person', role: null, person: null, reportsTo: null }));
  const agentNames = new Map(view.people.flatMap((p) => p.agents.map((a) => [a.id, a.display_name] as const)));
  const agents: Entry[] = view.people.flatMap((p) =>
    p.agents.map((a) => {
      const edge = reportingEdge(a);
      return {
      id: a.id,
      display_name: a.display_name,
      state: a.state,
      kind: 'agent' as const,
      role: null,
      person: { id: p.id, display_name: p.display_name, state: p.state },
      reportsTo: edge && edge.kind === 'agent' && edge.id !== a.id ? { id: edge.id, display_name: agentNames.get(edge.id) ?? null } : null,
      };
    }),
  );
  return [...people, ...agents];
}

export function visibleEntries(view: PeopleView, retired = false): Entry[] {
  const all = entries(view);
  const owners = new Set(all.filter(entry => entry.kind === 'agent' && entry.state !== 'retired').map(entry => entry.person?.id));
  return all.filter(entry => retired || entry.state !== 'retired' || entry.kind === 'person' && owners.has(entry.id));
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
