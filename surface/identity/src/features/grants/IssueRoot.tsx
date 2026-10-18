/** Give access: the administrator gives a person access to something outright (a root grant); the server alone decides who may. */
import { useState } from 'react';
import { api, Refused, request, useLoad } from '../../api';
import type { GrantModel, ResourceRef } from '../../generated/grants';
import type { PeopleView, MeView } from '../../generated';
import { Gate } from '../signin/Gate';
import { field, RecordedForm } from '../people/RecordedForm';
import { ErrorWords, STATUS } from '../people/Words';
import { Picker } from '../../shell/Picker';
import { MODE_HINTS, MODES, modeWords } from './mode-words';
import { resourceNames, resourceWords } from '../people/agent-resource-names';
import { LevelPicker } from './LevelPicker';

/** The resource choice that opens a box for an id Lys holds no grant on yet. */
const BY_ID = '\u0000by-id';

/**
 * The kinds a resource may be: each approved app's schema kinds under its name, then Lys's own kinds already held.
 * `roles` names each kind's roles (ACCESS-004 R1), kind to role to the actions it carries now.
 */
export interface KindGroup { label: string; kinds: string[]; roles?: Record<string, Record<string, string[]>> }

/** The roles the app schema names on `kind`, each with the actions it carries now. */
export const rolesOf = (kinds: KindGroup[], kind: string): Record<string, string[]> =>
  kinds.find((group) => group.kinds.includes(kind))?.roles?.[kind] ?? {};

/**
 * Whether the signed-in person is the root authority. The service answers the
 * whole directory only to the administrator, and the person bound to the
 * administrator's login is the root authority (GET /authority says so), so this
 * reads the service's answer and judges nothing.
 */
export const isRootAuthority = (people: PeopleView): boolean => people.scope === 'directory';

/** The refusal POST /grants/roots answers anyone but the root authority, said before any form is drawn. */
export const NOT_ROOT = new Refused(403, { refusal: 'RootAuthorityRefused', reason: 'only the administrator gives access directly' });

/** A kind in words: `cambium.channel` is "Channel" under its app's name, `project` is "Project". */
export const kindLabel = (kind: string): string => {
  const words = kind.slice(kind.indexOf('.') + 1).replaceAll('_', ' ').replaceAll('.', ' ');
  return words.charAt(0).toUpperCase() + words.slice(1);
};

/** The recipients a root grant's holder may pass it on to, as the two ticks say: other people, their agents, both or neither. */
const passTo = (data: FormData): ('person' | 'agent')[] => [
  ...(data.get('pass-people') === 'on' ? ['person' as const] : []), ...(data.get('delegate') === 'on' ? ['agent' as const] : [])];

/** Kinds whose every item the directory names: a person or agent is chosen by name, never typed as an id. */
const DIRECTORY_KINDS = new Set(['person', 'agent', 'identity']);

/** The items of `kind` Lys knows: those already granted, then, for a directory kind, everyone the directory lists. */
export function knownOf(kind: string, resources: ResourceRef[], people: PeopleView): ResourceRef[] {
  const held = resources.filter((resource) => resource.kind === kind);
  const listed = !DIRECTORY_KINDS.has(kind) ? [] : people.people.flatMap((person) => [
    ...(kind === 'agent' ? [] : [{ kind, id: person.id }]),
    ...(kind === 'person' ? [] : person.agents.map((agent) => ({ kind, id: agent.id }))),
  ]);
  const seen = new Set<string>();
  return [...held, ...listed].filter((resource) => !seen.has(resource.id) && seen.add(resource.id));
}

/**
 * One row you put things into: holder, kind of thing, which one, relation or role, mode, may pass on, expiry, Issue.
 * A role is sent as the relation; the grant is judged as the role's actions whenever it is judged, so a widening
 * the administrator approves reaches it.
 */
function Form({ people, me, model, kinds, resources, done }: { people: PeopleView; me: MeView; model: GrantModel; kinds: KindGroup[]; resources: ResourceRef[]; done: () => void }) {
  // A root grant has no expiry unless the issuer gives one.
  const [noExpiry, setNoExpiry] = useState(true);
  const [kind, setKind] = useState('');
  const [toAgent, setToAgent] = useState(false);
  const agents = people.people.find((person) => person.id === me.person.id)?.agents.filter((agent) => agent.state === 'active') ?? [];
  const relations = model.kinds?.[kind]?.relations ?? model.relations;
  const [mode, setMode] = useState<string>('outright');
  const [role, setRole] = useState('');
  const [level, setLevel] = useState('');
  const [which, setWhich] = useState('');
  const roles = rolesOf(kinds, kind);
  const names = resourceNames(people, [], null);
  const known = knownOf(kind, resources, people).map((resource) => ({ resource, words: resourceWords(resource, names).words }))
    .sort((a, b) => a.words.localeCompare(b.words));
  // Nothing is chosen before a kind is; with none of the kind known to Lys, the only way to name one is by its id in its app.
  const chosenWhich = !kind ? '' : known.length ? which : BY_ID;
  return <RecordedForm name="root-grant" title="Give access" submitLabel="Give access" drawn={{ symbol: 'add', word: 'Give access' }} done={done} change={(data) => {
    const relation = role || field(data, 'relation');
    const actions = role ? roles[role] : relations[relation];
    if (!actions) throw new Error(role ? 'Choose a role this kind names' : 'Choose what they may do');
    const chosen = field(data, 'kind');
    if (!kinds.some((group) => group.kinds.includes(chosen))) throw new Error('Choose a kind of thing from the list');
    const expires = noExpiry ? null : Date.parse(field(data, 'expires')) / 1000;
    if (expires !== null && !Number.isFinite(expires)) throw new Error('Enter a valid expiry');
    const holder = field(data, 'holder');
    if (toAgent && !agents.some((agent) => agent.id === holder)) throw new Error('Choose one of your active agents');
    return { path: toAgent ? '/grants' : '/grants/roots', body: {
      route: 'browser', ...(toAgent ? { source: 'administrator', recipient: holder, responsible: me.person.id } : { holder }),
      resource: { kind: chosen, id: field(data, 'resource') === BY_ID ? field(data, 'resource_id') : field(data, 'resource') }, relation,
      pass_on: passTo(data).length ? { kind: 'to', actions, recipients: passTo(data) } : { kind: 'use_only' },
      window: { starts_at: Math.floor(Date.now() / 1000), ends_at: expires },
      mode: field(data, 'mode'),
    } };
  }}>
    <p className="note">Give a person or one of your agents access. You can give to your agents directly as the administrator.</p>
    <div className="field">{toAgent ? 'Agent' : 'Person'}<select aria-label="Give to" value={toAgent ? 'agent' : 'person'} onChange={(event) => setToAgent(event.target.value === 'agent')}>
      <option value="person">A person</option><option value="agent">One of my agents</option>
    </select><Picker key={toAgent ? 'agent' : 'person'} name="holder" label={toAgent ? 'Find one of your agents' : 'Find a person'} options={toAgent ? agents.map((agent) => ({ id: agent.id, name: agent.display_name })) : people.people.filter((p) => p.state !== 'retired').map((p) => ({ id: p.id, name: p.display_name, detail: STATUS[p.state] ?? p.state }))} /></div>
    <label className="field">Give them access to<select name="kind" required value={kind} onChange={(event) => { setKind(event.target.value); setRole(''); setWhich(''); setLevel(''); }}>
      <option value="" disabled>Choose a kind</option>
      {kinds.map((group) => <optgroup key={group.label} label={group.label}>{group.kinds.map((each) => <option key={each} value={each} title={each}>{kindLabel(each)}</option>)}</optgroup>)}
    </select></label>
    <label className="field">Which one<select name="resource" required value={chosenWhich} disabled={!kind} onChange={(event) => setWhich(event.target.value)}>
      <option value="" disabled>{kind ? 'Choose one' : 'Choose a kind first'}</option>
      {known.map(({ resource, words }) => <option key={resource.id} value={resource.id} title={resource.id}>{words}</option>)}
      {kind ? <option value={BY_ID}>{known.length ? 'Another one, by its id in its app' : 'One by its id in its app'}</option> : null}
    </select></label>
    {kind && chosenWhich === BY_ID ? <label className="field">Its id<input name="resource_id" required autoComplete="off" /><span className="hint">{known.length ? 'The id the app itself gives it.' : 'Lys knows none of these by name yet. Enter the id the app itself gives it.'}</span></label> : null}
    <LevelPicker model={model} relations={relations} value={role ? '' : level} onChange={setLevel} name={role ? undefined : 'relation'} disabled={!!role} />
    {Object.keys(roles).length ? <label className="field">Role in the app<select name="role" value={role} onChange={(event) => setRole(event.target.value)} aria-label="Role">
      <option value="">No role: what is chosen above</option>
      {Object.entries(roles).map(([name, carried]) => <option key={name} value={name}>{name} · {carried.join(', ')}</option>)}
    </select><span className="hint">Gives whatever the app's role covers, including anything the app adds to it later once you approve that change.</span></label> : null}
    <div className="field">May they pass it on<label className="tick"><input name="pass-people" type="checkbox" /> to other people</label><label className="tick"><input name="delegate" type="checkbox" /> to their agents</label></div>
    <div className="field">Expires (your local time)<div className="expires"><input name="expires" type="datetime-local" aria-label="Expires" required={!noExpiry} disabled={noExpiry} />
      <label className="tick"><input type="checkbox" checked={noExpiry} onChange={(event) => setNoExpiry(event.target.checked)} /> No expiry</label></div></div>
    <details className="field"><summary>More</summary>
      <label className="field">Mode<select name="mode" required value={mode} onChange={(event) => setMode(event.target.value)}>{MODES.map((each) => <option key={each} value={each}>{modeWords(each)}</option>)}</select>
        <span className="hint">{MODE_HINTS[mode as keyof typeof MODE_HINTS] ?? ''}</span></label>
    </details>
  </RecordedForm>;
}

type AppRow = { id: string; name: string; state: string };
type SchemaAnswer = { app: string; version: number; schema: { kinds?: Record<string, { roles?: Record<string, string[]> }> } | null };

/**
 * The kinds the form offers: each approved app's schema kinds (GET /apps/{id}/schema),
 * then the kinds of the resources Lys already holds grants on that no app's schema names.
 * Lys never reads an app's own records, so the ids offered are the ones Lys already holds.
 */
export async function readKinds(resources: ResourceRef[]): Promise<KindGroup[]> {
  const { apps } = await request<{ apps: AppRow[] }>('/apps');
  const approved = apps.filter((app) => app.state === 'approved');
  const schemas = await Promise.all(approved.map((app) => request<SchemaAnswer>('/apps/' + encodeURIComponent(app.id) + '/schema')));
  const groups: KindGroup[] = approved.map((app, n) => {
    const declared = schemas[n]?.schema?.kinds ?? {};
    const roles = Object.fromEntries(Object.entries(declared).filter(([, body]) => body?.roles).map(([each, body]) => [each, body.roles ?? {}]));
    return { label: app.name, kinds: Object.keys(declared).sort(), roles };
  }).filter((group) => group.kinds.length);
  const named = new Set(groups.flatMap((group) => group.kinds));
  const held = [...new Set(resources.map((resource) => resource.kind))].filter((each) => !named.has(each)).sort();
  return held.length ? [...groups, { label: 'Already granted in Lys', kinds: held }] : groups;
}

async function read(resources: ResourceRef[]) {
  const people = await api.people();
  if (!isRootAuthority(people)) throw NOT_ROOT;
  const [me, model, kinds] = await Promise.all([api.me(), api.model(), readKinds(resources)]);
  return { people, me, model, kinds };
}

/** The Give access form, shown on the Access page above the grants. Anyone but the administrator is told so in words and given no form. */
export function IssueRoot({ resources, done = () => undefined }: { resources: ResourceRef[]; done?: () => void }) {
  const load = useLoad(() => read(resources), 'root-grant');
  return <Gate load={load} title="Give access" renderError={(problem) => <ErrorWords problem={problem} />} ok={(data) => <Form {...data} resources={resources} done={done} />} />;
}
