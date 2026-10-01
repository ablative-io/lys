import type { PeopleView } from '../../generated';
import type { ResourceRef } from '../../generated/grants';
import type { NetworkView } from '../network/contract';
import type { Team } from '../teams/contract';

const key = (kind: string, id: string) => kind + '\0' + id;
export function resourceNames(people: PeopleView, teams: Team[], network: NetworkView | null): Map<string, string> {
  const names = new Map<string, string>();
  for (const person of people.people) {
    names.set(key('person', person.id), person.display_name); names.set(key('identity', person.id), person.display_name);
    for (const agent of person.agents) { names.set(key('agent', agent.id), agent.display_name); names.set(key('identity', agent.id), agent.display_name); }
  }
  for (const team of teams) names.set(key('team', team.id), team.name);
  for (const machine of network?.machines ?? []) { names.set(key('machine', machine.id), machine.name); names.set(key('computer', machine.id), machine.name); }
  return names;
}
export function resourceWords(resource: ResourceRef, names: Map<string, string>): { words: string; unnamed: boolean } {
  const name = names.get(key(resource.kind, resource.id));
  const kind = resource.kind.replaceAll('_', ' ');
  if (name) return { words: resource.kind === 'identity' ? name + '’s identity' : resource.kind === 'person' || resource.kind === 'agent' ? 'the ' + kind + ' ' + name : 'the ' + name + ' ' + (resource.kind === 'machine' ? 'computer' : kind), unnamed: false };
  const short = resource.id.length > 12 ? resource.id.slice(0, 4) + '…' + resource.id.slice(-4) : resource.id;
  return { words: (/^[aeiou]/i.test(kind) ? 'an ' : 'a ') + kind + ', ' + short, unnamed: true };
}
