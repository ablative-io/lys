/** Root grants are requested explicitly; the server alone decides who may issue one. */
import { useState } from 'react';
import { api, Refused, request, useLoad } from '../../api';
import type { GrantModel, ResourceRef } from '../../generated/grants';
import type { PeopleView } from '../../generated';
import { Gate } from '../signin/Gate';
import { field, RecordedForm } from '../people/RecordedForm';
import { ErrorWords } from '../people/Words';
import { Picker } from '../../shell/Picker';
import { MODE_HINTS, MODES, modeWords } from './mode-words';

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
export const NOT_ROOT = new Refused(403, { refusal: 'RootAuthorityRefused', reason: 'you are not the root authority, and only it issues a root grant' });

/**
 * One row you put things into: holder, kind of thing, which one, relation or role, mode, may pass on, expiry, Issue.
 * A role is sent as the relation; the grant is judged as the role's actions whenever it is judged, so a widening
 * the administrator approves reaches it.
 */
function Form({ people, model, kinds, resources, done }: { people: PeopleView; model: GrantModel; kinds: KindGroup[]; resources: ResourceRef[]; done: () => void }) {
  const [noExpiry, setNoExpiry] = useState(false);
  const [kind, setKind] = useState('');
  const [mode, setMode] = useState<string>('outright');
  const [role, setRole] = useState('');
  const roles = rolesOf(kinds, kind);
  return <RecordedForm name="root-grant" title="Issue root grant" submitLabel="Issue" drawn={{ symbol: 'add', word: 'Issue' }} done={done} change={(data) => {
    const relation = role || field(data, 'relation');
    const actions = role ? roles[role] : model.relations[relation];
    if (!actions) throw new Error(role ? 'Choose a role this kind names' : 'Select a relation from the permission model');
    const chosen = field(data, 'kind');
    if (!kinds.some((group) => group.kinds.includes(chosen))) throw new Error('Choose a kind of thing from the list');
    const expires = noExpiry ? null : Date.parse(field(data, 'expires')) / 1000;
    if (expires !== null && !Number.isFinite(expires)) throw new Error('Enter a valid expiry');
    return { path: '/grants/roots', body: {
      route: 'browser', holder: field(data, 'holder'),
      resource: { kind: chosen, id: field(data, 'resource') }, relation,
      pass_on: data.get('delegate') === 'on' ? { kind: 'to', actions, recipients: ['agent'] } : { kind: 'use_only' },
      window: { starts_at: Math.floor(Date.now() / 1000), ends_at: expires },
      mode: field(data, 'mode'),
    } };
  }}>
    <p className="note">Only the directory's root authority may issue this grant. It is given to a person, who may pass it to their agents only if you allow it here.</p>
    <div className="field">Holder<Picker name="holder" label="Find a person" options={people.people.map((p) => ({ id: p.id, name: p.display_name, detail: p.state }))} /></div>
    <label className="field">Kind of thing<select name="kind" required value={kind} onChange={(event) => { setKind(event.target.value); setRole(''); }}>
      <option value="" disabled>Choose a kind</option>
      {kinds.map((group) => <optgroup key={group.label} label={group.label}>{group.kinds.map((each) => <option key={each} value={each}>{each}</option>)}</optgroup>)}
    </select></label>
    <label className="field">Which one<input name="resource" required list="root-ids" autoComplete="off" /></label>
    <datalist id="root-ids">{resources.filter((resource) => resource.kind === kind).map((resource) => <option key={resource.kind + ':' + resource.id} value={resource.id} />)}</datalist>
    <label className="field">Relation<select name="relation" required={!role} disabled={!!role} defaultValue=""><option value="" disabled>Choose a relation</option>{Object.entries(model.relations).map(([relation, actions]) => <option key={relation} value={relation}>{relation} · {actions.join(', ')}</option>)}</select></label>
    <label className="field">Mode<select name="mode" required value={mode} onChange={(event) => setMode(event.target.value)}>{MODES.map((each) => <option key={each} value={each}>{modeWords(each)}</option>)}</select>
      <span className="hint">{MODE_HINTS[mode as keyof typeof MODE_HINTS] ?? ''}</span></label>
    {Object.keys(roles).length ? <label className="field">Role<select name="role" value={role} onChange={(event) => setRole(event.target.value)} aria-label="Role">
      <option value="">No role: the relation above</option>
      {Object.entries(roles).map(([name, carried]) => <option key={name} value={name}>{name} · {carried.join(', ')}</option>)}
    </select><span className="hint">A role gives what it carries whenever it is checked, so a widening you approve reaches this grant.</span></label> : null}
    <div className="field">May pass on<label className="tick"><input name="delegate" type="checkbox" /> to their agents</label></div>
    <div className="field">Expires (your local time)<div className="expires"><input name="expires" type="datetime-local" aria-label="Expires" required={!noExpiry} disabled={noExpiry} />
      <label className="tick"><input type="checkbox" checked={noExpiry} onChange={(event) => setNoExpiry(event.target.checked)} /> no expiry</label></div></div>
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
  const [model, kinds] = await Promise.all([api.model(), readKinds(resources)]);
  return { people, model, kinds };
}

/** The form that issues a root grant, shown in the Access page above the grants. Anyone but the root authority is told so by name and given no form. */
export function IssueRoot({ resources, done = () => undefined }: { resources: ResourceRef[]; done?: () => void }) {
  const load = useLoad(() => read(resources), 'root-grant');
  return <Gate load={load} title="Issue access" renderError={(problem) => <ErrorWords problem={problem} />} ok={(data) => <Form {...data} resources={resources} done={done} />} />;
}
