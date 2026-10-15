/**
 * What a schema change does to roles (ACCESS-004 R1), in the words the
 * service's schema diff uses: "adds seat_retire to administrator". The Apps
 * screen shows these beside a change an app proposed, so the administrator
 * sees each widening by name before approving or declining it.
 */

type Roles = Record<string, Record<string, string[]>>;
type Shape = { kinds?: Record<string, { roles?: Record<string, string[]>; agents?: string[]; hot?: string[] } | null> } | null;

/** Each kind's roles, kind to role to the actions it carries. */
export function rolesIn(schema: unknown): Roles {
  const kinds = (schema as Shape)?.kinds ?? {};
  return Object.fromEntries(Object.entries(kinds).map(([kind, body]) => [kind, body?.roles ?? {}]));
}

/** Every role, agent and hot action change, so approval names changes to who may act and how an action is decided. */
export function roleChanges(before: unknown, after: unknown): string[] {
  const was = rolesIn(before);
  const now = rolesIn(after);
  const words: string[] = [];
  for (const kind of [...new Set([...Object.keys(was), ...Object.keys(now)])].sort()) {
    const old = was[kind] ?? {};
    const next = now[kind] ?? {};
    for (const role of [...new Set([...Object.keys(old), ...Object.keys(next)])].sort()) {
      const held = old[role];
      const given = next[role];
      if (!held && given) words.push('adds the role ' + role + ' to ' + kind + ', carrying ' + given.join(', '));
      else if (held && !given) words.push('removes the role ' + role + ' from ' + kind);
      else if (held && given) {
        for (const action of given.filter((each) => !held.includes(each))) words.push('adds ' + action + ' to ' + role);
        for (const action of held.filter((each) => !given.includes(each))) words.push('removes ' + action + ' from ' + role);
      }
    }
    const previous = (before as Shape)?.kinds?.[kind];
    const proposed = (after as Shape)?.kinds?.[kind];
    const oldAgents = previous?.agents ?? [];
    const nextAgents = proposed?.agents ?? [];
    for (const action of nextAgents.filter((each) => !oldAgents.includes(each)).sort()) words.push('agents may now hold ' + action + ' on ' + kind);
    for (const action of oldAgents.filter((each) => !nextAgents.includes(each)).sort()) words.push('agents may no longer hold ' + action + ' on ' + kind);
    const oldHot = previous?.hot ?? [];
    const nextHot = proposed?.hot ?? [];
    for (const action of nextHot.filter((each) => !oldHot.includes(each)).sort()) words.push('makes ' + action + ' hot on ' + kind + ' (decided from the pass alone)');
    for (const action of oldHot.filter((each) => !nextHot.includes(each)).sort()) words.push('makes ' + action + ' deliberate on ' + kind);
  }
  return words;
}
