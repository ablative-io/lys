/** Give access: the administrator gives a person access to something outright (a root grant); the server alone decides who may. */
import { useState } from 'react';
import { useSearchParams } from 'react-router';
import { api, Refused, request, useLoad } from '../../api';
import type { GrantModel, ResourceRef } from '../../generated/grants';
import type { PeopleView, MeView, OrganisationSummary } from '../../generated';
import { Gate } from '../signin/Gate';
import { field, RecordedForm } from '../people/RecordedForm';
import type { Change } from '../people/RecordedForm';
import { ErrorWords, STATUS } from '../people/Words';
import { Picker } from '../../shell/Picker';
import { MODE_HINTS, MODES, modeWords } from './mode-words';
import { key, resourceNames, resourceWords } from '../people/agent-resource-names';
import { LEVEL_NAMES, LevelPicker } from './LevelPicker';
import { actionWords } from './action-words';
import { organisationActs, ORGANISATION_GRANT } from './organisation-acts';
import type { OrganisationActs } from './organisation-acts';

/** The resource choice that opens a box for an id Lys holds no grant on yet. */
const BY_ID = '\u0000by-id';

/**
 * Lys's own organisation kind (24 October 2026): offered by name to whoever may give access directly, so standing in
 * the organisation, administrator among them, is given from this form like anything else, to a person or an agent.
 */
export const ORGANISATION_KIND = 'organisation';

/**
 * The relations the form offers on `kind`: for the organisation, its standing relations alone, those carrying only
 * organisation acts (the levels say nothing about an organisation); for any other kind, the model's own.
 */
export function relationsOf(model: GrantModel, kind: string): Record<string, string[]> {
  const all = model.kinds?.[kind]?.relations ?? model.relations;
  if (kind !== ORGANISATION_KIND) return all;
  const standing = Object.fromEntries(Object.entries(all).filter(([, actions]) => actions.length > 0 && actions.every((action) => action.startsWith('organisation.'))));
  return Object.keys(standing).length ? standing : all;
}

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

/**
 * Whether the signed-in person may give access directly (ACCESS-009 R4): a standing grant of theirs on their
 * organisation carries organisation.grant, or they are the install's root authority, who roots its own kinds.
 */
export const mayGiveAccess = (acts: OrganisationActs, people: PeopleView): boolean => acts.grant || isRootAuthority(people);

/**
 * The refusal said to a person who may not give access directly, before any form is drawn: within an organisation it
 * names the right they do not hold and who gives access there; outside one it is the install's own refusal.
 */
export const notRoot = (acts: OrganisationActs): Refused => acts.organisation
  ? new Refused(403, { refusal: 'RootAuthorityRefused', reason: `you hold no ${ORGANISATION_GRANT} on ${acts.organisation.display_name}, so you cannot give access directly; whoever holds ${ORGANISATION_GRANT} there gives it, and access you hold that may be passed on is given from a person's file` })
  : NOT_ROOT;

/** A kind in words: `fixture_product.channel` is "Channel" under its app's name, `project` is "Project". */
export const kindLabel = (kind: string): string => {
  const words = kind.slice(kind.indexOf('.') + 1).replaceAll('_', ' ').replaceAll('.', ' ');
  return words.charAt(0).toUpperCase() + words.slice(1);
};

/** The recipients a root grant's holder may pass it on to, as the two ticks say: other people, their agents, both or neither. */
const passTo = (data: FormData): ('person' | 'agent')[] => [
  ...(data.get('pass-people') === 'on' ? ['person' as const] : []), ...(data.get('delegate') === 'on' ? ['agent' as const] : [])];

/** Kinds whose every item the directory names: a person or agent is chosen by name, never typed as an id. */
const DIRECTORY_KINDS = new Set(['person', 'agent', 'identity']);

/**
 * The items of `kind` Lys knows: the person's own organisation first, then those already granted, then, for a directory
 * kind, everyone the directory lists.
 */
export function knownOf(kind: string, resources: ResourceRef[], people: PeopleView, organisation: OrganisationSummary | null = null): ResourceRef[] {
  const own = kind === ORGANISATION_KIND && organisation ? [{ kind, id: organisation.id }] : [];
  const held = resources.filter((resource) => resource.kind === kind);
  const listed = !DIRECTORY_KINDS.has(kind) ? [] : people.people.flatMap((person) => [
    ...(kind === 'agent' ? [] : [{ kind, id: person.id }]),
    ...(kind === 'person' ? [] : person.agents.map((agent) => ({ kind, id: agent.id }))),
  ]);
  const seen = new Set<string>();
  return [...own, ...held, ...listed].filter((resource) => !seen.has(resource.id) && seen.add(resource.id));
}

/**
 * One row you put things into: holder, kind of thing, which one, relation or role, mode, may pass on, expiry, Issue.
 * A role is sent as the relation; the grant is judged as the role's actions whenever it is judged, so a widening
 * the administrator approves reaches it.
 */
function Form({ people, me, model, kinds, resources, done }: { people: PeopleView; me: MeView; model: GrantModel; kinds: KindGroup[]; resources: ResourceRef[]; done: () => void }) {
  // A root grant has no expiry unless the issuer gives one.
  const [noExpiry, setNoExpiry] = useState(true);
  // `?app=<id>` (Apps' "Give people access") opens on that app's first kind, so the form starts where the person meant to be.
  const [params] = useSearchParams();
  const app = params.get('app');
  const [kind, setKind] = useState(() => (app ? kinds.flatMap((group) => group.kinds).find((each) => each.startsWith(app + '.')) : undefined) ?? '');
  const [toAgent, setToAgent] = useState(false);
  const agents = people.people.find((person) => person.id === me.person.id)?.agents.filter((agent) => agent.state === 'active') ?? [];
  const relations = relationsOf(model, kind);
  const [mode, setMode] = useState<string>('outright');
  const [role, setRole] = useState('');
  const [level, setLevel] = useState('');
  // What the last give did, said above a fresh form: a form left filled after a give invites a second, identical grant.
  const [given, setGiven] = useState<{ holder: string; name: string; words: string } | null>(null);
  const [round, setRound] = useState(0);
  const [which, setWhich] = useState('');
  const roles = rolesOf(kinds, kind);
  const names = resourceNames(people, [], null);
  const organisation = me.organisation ?? null;
  if (organisation) names.set(key(ORGANISATION_KIND, organisation.id), organisation.display_name);
  const known = knownOf(kind, resources, people, organisation).map((resource) => ({ resource, words: resourceWords(resource, names).words }))
    .sort((a, b) => a.words.localeCompare(b.words));
  // Nothing is chosen before a kind is; with none of the kind known to Lys, the only way to name one is by its id in its app.
  const chosenWhich = !kind ? '' : known.length ? which : BY_ID;
  /** What a give did, in words: who can now do what, on what. The holder's name and the thing's are the directory's, never an id where a name is known. */
  const words = (asked: Change): string => {
    const body = asked.body as { holder?: string; recipient?: string; resource: ResourceRef; relation: string };
    const holder = body.holder ?? body.recipient ?? '';
    const name = people.people.flatMap((person) => [person, ...person.agents]).find((each) => each.id === holder)?.display_name ?? 'They';
    const thing = resourceWords(body.resource, names).words;
    const app = rolesOf(kinds, body.resource.kind)[body.relation];
    if (body.resource.kind === ORGANISATION_KIND) return `${name} is now ${body.relation} of ${thing}.`;
    if (LEVEL_NAMES.includes(body.relation)) return `${name} can now ${({ viewer: 'view', editor: 'edit', owner: 'own' } as Record<string, string>)[body.relation]} ${thing}.`;
    if (app) return `${name} now has the app's ${body.relation} role on ${thing}.`;
    const words = actionWords(model, body.resource, relationsOf(model, body.resource.kind)[body.relation] ?? [body.relation]);
    // The model's sentences say "this agent" or "this resource": the thing given takes that place, so it is never named twice.
    const sentence = words.charAt(0).toLowerCase() + words.slice(1);
    return `${name} can now ${/\bthis [a-z]+/.test(sentence) ? sentence.replace(/\bthis [a-z]+/, thing) : sentence + ', on ' + thing}.`;
  };
  const said = (asked: Change): string => {
    const sentence = words(asked);
    const body = asked.body as { holder?: string; recipient?: string };
    const holder = body.holder ?? body.recipient ?? '';
    setGiven({ holder, name: people.people.flatMap((person) => [person, ...person.agents]).find((each) => each.id === holder)?.display_name ?? 'them', words: sentence });
    setKind(''); setLevel(''); setRole(''); setWhich(''); setRound((value) => value + 1);
    return sentence;
  };
  return <>{given ? <p className="next-step" role="status">{given.words} <a href={'#/file/' + encodeURIComponent(given.holder) + '/access'}>See what {given.name} can do now</a>, or give more access below.</p> : null}
  <RecordedForm key={round} name="root-grant" title="Give access" submitLabel="Give access" drawn={{ symbol: 'add', word: 'Give access' }} done={done} success={said} change={(data) => {
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
  </RecordedForm></>;
}

type AppRow = { id: string; name: string; state: string };
type SchemaAnswer = { app: string; version: number; schema: { kinds?: Record<string, { roles?: Record<string, string[]> }> } | null };

/**
 * The kinds the form offers: each approved app's schema kinds (GET /apps/{id}/schema), then Lys's own organisation
 * when the signed-in person belongs to one, then the kinds of the resources Lys already holds grants on that nothing
 * above names. Lys never reads an app's own records, so the ids offered are the ones Lys already holds.
 */
export async function readKinds(resources: ResourceRef[], organisation: OrganisationSummary | null = null): Promise<KindGroup[]> {
  const { apps } = await request<{ apps: AppRow[] }>('/apps');
  const approved = apps.filter((app) => app.state === 'approved');
  const schemas = await Promise.all(approved.map((app) => request<SchemaAnswer>('/apps/' + encodeURIComponent(app.id) + '/schema')));
  const groups: KindGroup[] = approved.map((app, n) => {
    const declared = schemas[n]?.schema?.kinds ?? {};
    const roles = Object.fromEntries(Object.entries(declared).filter(([, body]) => body?.roles).map(([each, body]) => [each, body.roles ?? {}]));
    return { label: app.name, kinds: Object.keys(declared).sort(), roles };
  }).filter((group) => group.kinds.length);
  const lys: KindGroup[] = organisation ? [{ label: 'Lys', kinds: [ORGANISATION_KIND] }] : [];
  const named = new Set([...groups, ...lys].flatMap((group) => group.kinds));
  const held = [...new Set(resources.map((resource) => resource.kind))].filter((each) => !named.has(each)).sort();
  return [...groups, ...lys, ...(held.length ? [{ label: 'Already granted in Lys', kinds: held }] : [])];
}

async function read(resources: ResourceRef[]) {
  const [people, me, list] = await Promise.all([api.people(), api.me(), api.grants()]);
  const acts = organisationActs(me, list.grants);
  if (!mayGiveAccess(acts, people)) throw notRoot(acts);
  const [model, kinds] = await Promise.all([api.model(), readKinds(resources, me.organisation ?? null)]);
  return { people, me, model, kinds };
}

/** The Give access form, shown on the Access page above the grants. Anyone but the administrator is told so in words and given no form. */
export function IssueRoot({ resources, done = () => undefined }: { resources: ResourceRef[]; done?: () => void }) {
  const load = useLoad(() => read(resources), 'root-grant');
  return <Gate load={load} title="Give access" renderError={(problem) => <ErrorWords problem={problem} />} ok={(data) => <Form {...data} resources={resources} done={done} />} />;
}
