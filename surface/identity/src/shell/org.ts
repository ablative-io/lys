/** The organisation as the teams record it: a tree of teams, each with a lead and members, people and agents. */

/** A team as /teams serves it; parent and lead are absent in stores written before nesting. */
export interface OrgTeam {
  id: string;
  name: string;
  owner: string;
  parent?: string | null;
  lead?: string | null;
  members: string[];
  state: 'active' | 'retired';
}

/** Whose records a screen shows: the caller's own, one team and every team under it, or everyone the caller may see. */
export type Whose = { kind: 'mine' } | { kind: 'team'; team: string } | { kind: 'all' };

export const whoseParam = (whose: Whose): string => whose.kind === 'team' ? 'team:' + whose.team : whose.kind;

export function readWhose(value: string | null, admin: boolean): Whose {
  if (value?.startsWith('team:')) return { kind: 'team', team: value.slice(5) };
  if (value === 'all' && admin) return { kind: 'all' };
  if (value === 'mine') return { kind: 'mine' };
  return admin ? { kind: 'all' } : { kind: 'mine' };
}

/** Active teams in tree order, each with its depth, siblings by name. */
export function treeOrder(teams: OrgTeam[]): { team: OrgTeam; depth: number }[] {
  const active = teams.filter((team) => team.state === 'active');
  const known = new Set(active.map((team) => team.id));
  const children = (parent: string | null) => active
    .filter((team) => (team.parent && known.has(team.parent) ? team.parent : null) === parent)
    .sort((left, right) => left.name.localeCompare(right.name));
  const out: { team: OrgTeam; depth: number }[] = [];
  const walk = (parent: string | null, depth: number) => {
    for (const team of children(parent)) { out.push({ team, depth }); walk(team.id, depth + 1); }
  };
  walk(null, 0);
  return out;
}

/** The team and every team nested under it. */
export function subtree(teams: OrgTeam[], root: string): Set<string> {
  const found = new Set([root]);
  let grew = true;
  while (grew) {
    grew = false;
    for (const team of teams) {
      if (team.parent && found.has(team.parent) && !found.has(team.id)) { found.add(team.id); grew = true; }
    }
  }
  return found;
}

/**
 * One record a list shows: its id, the person it answers to when it is an agent,
 * and any others whose teams it belongs with, such as the agents that may start on a computer.
 */
export interface Held { id: string; person: string | null; also?: string[] }

/** The teams a record belongs to: its own memberships, and for an agent its person's. */
export function teamsOf(teams: OrgTeam[], held: Held): OrgTeam[] {
  const ids = [held.id, ...(held.person === null ? [] : [held.person]), ...(held.also ?? [])];
  const people = new Set(ids);
  return teams.filter((team) => team.state === 'active' && team.members.some((member) => people.has(member)));
}

/** Whether a record is inside the chosen scope. */
export function inWhose(whose: Whose, teams: OrgTeam[], me: string, held: Held): boolean {
  if (whose.kind === 'all') return true;
  if (whose.kind === 'mine') return held.id === me || held.person === me;
  const scope = subtree(teams, whose.team);
  return teamsOf(teams, held).some((team) => scope.has(team.id));
}

/** A group of rows under one team, or under no team. */
export interface Group<T> {
  id: string;
  name: string;
  /** The lead's name, when the team names one. */
  lead: string | null;
  depth: number;
  /** The records that belong to this team itself. */
  items: T[];
  /** The records of this team and every team under it, each once. */
  within: T[];
}

/**
 * Group records by the teams they belong to, in tree order, within the scope.
 * A record in two teams is listed under each; a team with none of its own still
 * heads the teams under it; records in no team come last.
 */
export function groupByTeam<T>(items: T[], held: (item: T) => Held, teams: OrgTeam[], whose: Whose, name: (id: string) => string): Group<T>[] {
  const scope = whose.kind === 'team' ? subtree(teams, whose.team) : null;
  const order = treeOrder(teams).filter(({ team }) => scope === null || scope.has(team.id));
  const top = scope === null ? 0 : order[0]?.depth ?? 0;
  const groups: Group<T>[] = order.map(({ team, depth }) => ({ id: team.id, name: team.name, lead: team.lead ? name(team.lead) : null, depth: depth - top, items: [], within: [] }));
  const byId = new Map(groups.map((group) => [group.id, group]));
  const loose: T[] = [];
  for (const item of items) {
    const mine = teamsOf(teams, held(item)).filter((team) => byId.has(team.id));
    if (!mine.length) loose.push(item);
    for (const team of mine) byId.get(team.id)?.items.push(item);
  }
  for (const group of groups) {
    const below = subtree(teams, group.id);
    group.within = [...new Set(groups.filter((each) => below.has(each.id)).flatMap((each) => each.items))];
  }
  const kept = groups.filter((group) => group.within.length);
  return loose.length && scope === null ? [...kept, { id: '', name: 'In no team', lead: null, depth: 0, items: loose, within: loose }] : kept;
}
