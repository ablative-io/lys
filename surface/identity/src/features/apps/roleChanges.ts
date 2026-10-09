/**
 * What a schema change does to roles (ACCESS-004 R1), in the words the
 * service's schema diff uses: "adds seat_retire to administrator". The Apps
 * screen shows these beside a change an app proposed, so the administrator
 * sees each widening by name before approving or declining it.
 */

type Roles = Record<string, Record<string, string[]>>;
type Shape = { kinds?: Record<string, { roles?: Record<string, string[]> } | null> } | null;

/** Each kind's roles, kind to role to the actions it carries. */
export function rolesIn(schema: unknown): Roles {
  const kinds = (schema as Shape)?.kinds ?? {};
  return Object.fromEntries(Object.entries(kinds).map(([kind, body]) => [kind, body?.roles ?? {}]));
}

/** Every role change from `before` to `after`, one sentence each, kinds and roles in name order. */
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
  }
  return words;
}
