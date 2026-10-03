import { readTogether } from '../../reads';
/** Metadata-only secrets listing; authentication and visibility belong to the broker adapter. */
import type { ReactNode } from 'react';
import { Listing } from '../../shell/Listing';
import type { Column } from '../../shell/Listing';
import { groupByTeam, inWhose } from '../../shell/org';
import type { Held, OrgTeam, Whose } from '../../shell/org';
import { useWhose, WhoseSelect } from '../../shell/Whose';
import { readTeams } from '../teams/Teams';
import { api, useLoad } from '../../api';
import type { PeopleView } from '../../generated';
import { entries } from '../people/directory';
import { Gate } from '../signin/Gate';
import { IdentityName, ReadFailure } from '../signin/words';

/** The existing /_lys/secrets response in lys-secrets/bin/lys-secrets/view.rs. */
export interface SecretListing {
  secrets: {
    name: string;
    class: 'credential' | 'key' | 'memory' | 'oauth';
    owner: string;
    sequence: number;
    upstream: string | null;
    header: string | null;
  }[];
}

type Secret = SecretListing['secrets'][number];

export function Secrets({ read }: { read: () => Promise<SecretListing> }) {
  const load = useLoad(read, 'secrets');
  const people = useLoad(() => readTogether({ view: api.people(), me: api.me(), teams: readTeams().catch(() => []) }), 'secret-people');
  const admin = people.status === 'ok' && people.data.view.scope === 'directory';
  const [whose, setWhose] = useWhose(admin);
  return <>
    <p className="sub">Find the secrets you are allowed to see. Their values stay in Lys secret storage.</p>
    {people.status === 'refused' ? <ReadFailure error={people.refused} subject="secret owners’ names" /> : null}
    <Gate load={load} title="your secrets" renderError={(error) => <ReadFailure error={error} subject="your secrets" administrator={admin} />} ok={(listing) => people.status === 'ok'
      ? <SecretRows listing={listing} people={people.data.view} teams={people.data.teams} me={people.data.me.person.id} whose={whose}
        tools={<WhoseSelect whose={whose} set={setWhose} teams={people.data.teams} admin={admin} />} />
      : <SecretRows listing={listing} />} />
  </>;
}

/** The secrets as the shared list: grouped by their owner's team, searched by name, kind or owner. */
export function SecretRows({ listing, people, teams = [], me = '', whose = { kind: 'all' }, tools }: { listing: SecretListing; people?: PeopleView; teams?: OrgTeam[]; me?: string; whose?: Whose; tools?: ReactNode }) {
  const names = new Map(people ? entries(people).map((entry) => [entry.id, entry.display_name]) : []);
  const owners = new Map(people ? people.people.flatMap((person) => person.agents.map((agent) => [agent.id, person.id] as const)) : []);
  const held = (entry: Secret): Held => ({ id: entry.owner, person: owners.get(entry.owner) ?? null });
  const scoped = listing.secrets.filter((entry) => inWhose(whose, teams, me, held(entry)));
  const groups = groupByTeam(scoped, held, teams, whose, (id) => names.get(id) ?? 'someone outside your view');
  const columns: Column<Secret>[] = [
    { head: 'Name', cell: (entry) => entry.name },
    { head: 'Kind', cell: (entry) => <span className="sec">{entry.class}</span> },
    { head: 'Owner', cell: (entry) => <IdentityName id={entry.owner} people={people} /> },
    { head: 'Recorded change', cell: (entry) => <span className="sec">{entry.sequence}</span> },
  ];
  return <>
    <p className="note">Seeing a secret here does not give permission to use it or share that permission. Its value is never shown.</p>
    {!listing.secrets.length ? <p className="note">No secrets were returned for this account.</p> : null}
    <div className="body one">
    <Listing<Secret> groups={groups} columns={columns} id={(entry) => entry.name} href={(entry) => '#/secrets/entries?secret=' + encodeURIComponent(entry.name)}
      words={(entry) => entry.name + ' ' + entry.class + ' ' + (names.get(entry.owner) ?? '')} noun="secrets" holds={(items) => items.length + (items.length === 1 ? ' secret' : ' secrets')}
      selected={null} select={() => undefined} tools={tools} />
    </div>
  </>;
}
