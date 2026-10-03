/**
 * The permission template builder: a person makes an app's whole schema
 * here without typing any schema language. Kinds are added under the app's
 * prefix, each given its actions; relations are made and the actions each
 * carries are ticked; parents are chosen. A template starts the draft. The
 * kinds are drawn as a tree, each editable in place, with the test bench
 * beside them.
 *
 * The draft lives in this builder's own session store and nowhere else, so
 * no standing check sees it. Saving runs the service's dry run first, lists
 * what would be added and removed and every standing grant a change would
 * strand, and cannot save a change that strands grants. It saves through the
 * same routes an upload uses, so a schema built here and one uploaded through
 * the API are the same record, and an uploaded schema opens here to edit.
 */
import { useEffect, useState } from 'react';
import { Refused, operationId, send } from '../../api';
import { SchemaBench } from './SchemaBench';
import './schema-builder.css';

/** A kind as the schema JSON writes it. */
export interface SchemaKindJson { actions: string[]; relations?: Record<string, string[]>; parents?: string[] }
/** An app's schema as the API takes it. */
export interface SchemaJson { kinds: Record<string, SchemaKindJson> }
/** One relation of a draft kind. */
export interface DraftRelation { name: string; actions: string[] }
/** One kind of the draft, named within the app's prefix. */
export interface DraftKind { name: string; actions: string[]; relations: DraftRelation[]; parents: string[] }
/** The draft being built. */
export interface Draft { kinds: DraftKind[] }

/** What a change would do, as the dry run answers it. */
export interface Named { kind: string; name: string }
export interface Strand { kind: string; relation: string; count: number }
export interface SchemaCheck {
  current: number; next: number; applies: boolean; stranded: Strand[];
  diff: { kinds_added: string[]; kinds_removed: string[]; relations_added: Named[]; relations_removed: Named[]; actions_added: Named[]; actions_removed: Named[]; parents_added: Named[]; parents_removed: Named[] };
}

export { send };

/** The schema JSON the draft makes, every kind under `app`'s prefix. */
export function toSchema(app: string, draft: Draft): SchemaJson {
  const kinds: Record<string, SchemaKindJson> = {};
  for (const kind of draft.kinds) {
    kinds[app + '.' + kind.name] = {
      actions: [...kind.actions],
      relations: Object.fromEntries(kind.relations.map((relation) => [relation.name, kind.actions.filter((action) => relation.actions.includes(action))])),
      parents: kind.parents.map((parent) => app + '.' + parent),
    };
  }
  return { kinds };
}

/** The draft an uploaded schema opens as; a kind outside `app`'s prefix is named, never dropped. */
export function fromSchema(app: string, schema: unknown): Draft {
  const kinds = (schema as { kinds?: unknown } | null)?.kinds;
  if (!kinds || typeof kinds !== 'object') throw new Error('The schema holds no kinds to open.');
  const local = (name: string) => {
    if (!name.startsWith(app + '.')) throw new Error('The kind ' + name + ' is outside the prefix ' + app + ' and cannot be opened here.');
    return name.slice(app.length + 1);
  };
  return {
    kinds: Object.entries(kinds as Record<string, SchemaKindJson>).map(([name, kind]) => ({
      name: local(name),
      actions: [...(kind.actions ?? [])],
      relations: Object.entries(kind.relations ?? {}).map(([relation, actions]) => ({ name: relation, actions: [...actions] })),
      parents: (kind.parents ?? []).map(local),
    })),
  };
}

/** The templates a draft starts from. */
export const TEMPLATES: { id: string; label: string; draft: Draft }[] = [
  {
    id: 'owner-editor-viewer',
    label: 'Owner, editor and viewer on one kind',
    draft: { kinds: [{ name: 'document', actions: ['read', 'write', 'share'], relations: [{ name: 'owner', actions: ['read', 'write', 'share'] }, { name: 'editor', actions: ['read', 'write'] }, { name: 'viewer', actions: ['read'] }], parents: [] }] },
  },
  {
    id: 'workspace',
    label: 'A workspace whose members reach its channels',
    draft: { kinds: [
      { name: 'workspace', actions: ['read', 'write', 'admin'], relations: [{ name: 'owner', actions: ['read', 'write', 'admin'] }, { name: 'member', actions: ['read'] }], parents: [] },
      { name: 'channel', actions: ['read', 'write'], relations: [{ name: 'poster', actions: ['read', 'write'] }], parents: ['workspace'] },
    ] },
  },
  {
    id: 'team-scoped',
    label: 'Resources scoped to a team',
    draft: { kinds: [
      { name: 'team', actions: ['read', 'manage'], relations: [{ name: 'lead', actions: ['read', 'manage'] }, { name: 'member', actions: ['read'] }], parents: [] },
      { name: 'project', actions: ['read', 'write'], relations: [{ name: 'contributor', actions: ['read', 'write'] }], parents: ['team'] },
    ] },
  },
];

const copy = (draft: Draft): Draft => JSON.parse(JSON.stringify(draft)) as Draft;
const storageKey = (app: string) => 'lys.schema-draft.' + app;

function kept(app: string, initial: Draft): Draft {
  const stored = sessionStorage.getItem(storageKey(app));
  if (!stored) return initial;
  try { return JSON.parse(stored) as Draft; } catch { return initial; }
}

/** The builder for `app`'s schema at `version`, opened from `initial`. */
export function SchemaBuilder({ app, version, initial, onSaved, register }: { app: string; version: number | null; initial?: SchemaJson; onSaved?: (message: string) => void; register?: (schema: SchemaJson) => Promise<void> }) {
  const [opened] = useState(() => { try { return { draft: initial ? fromSchema(app, initial) : { kinds: [] }, error: '' }; } catch (error) { return { draft: { kinds: [] }, error: String(error) }; } });
  const [draft, setDraft] = useState<Draft>(() => kept(app, opened.draft));
  const [check, setCheck] = useState<SchemaCheck | null>(null);
  const [refusal, setRefusal] = useState<string>(opened.error);
  const [busy, setBusy] = useState(false);
  useEffect(() => { sessionStorage.setItem(storageKey(app), JSON.stringify(draft)); }, [app, draft]);
  const change = (edit: (next: Draft) => void) => { const next = copy(draft); edit(next); setDraft(next); setCheck(null); };
  const schema = toSchema(app, draft);
  const refused = (error: unknown) => setRefusal(error instanceof Refused ? error.refusal.refusal + ': ' + error.refusal.reason : String(error));
  const dryRun = async () => {
    setBusy(true); setRefusal('');
    try { setCheck(await send<SchemaCheck>('POST', '/apps/' + encodeURIComponent(app) + '/schema/check', { replaces: version, schema })); } catch (error) { refused(error); }
    setBusy(false);
  };
  const save = async () => {
    setBusy(true); setRefusal('');
    try {
      if (version === null) { await register?.(schema); } else { await send('PUT', '/apps/' + encodeURIComponent(app) + '/schema', { operation: operationId(), replaces: version, schema }); }
      sessionStorage.removeItem(storageKey(app)); setCheck(null);
      onSaved?.(version === null ? 'The app is registered and waits for approval.' : 'The schema was saved.');
    } catch (error) { refused(error); }
    setBusy(false);
  };
  return <section className="sb" aria-label={'Permission template for ' + app}>
    <div className="sb-templates" role="group" aria-label="Start from a template">
      <span className="note">Start from</span>
      {TEMPLATES.map((template) => <button type="button" className="sb-chip" key={template.id} onClick={() => change((next) => { next.kinds = copy(template.draft).kinds; })}>{template.label}</button>)}
    </div>
    <div className="sb-layout">
      <div className="sb-tree" aria-label="Kinds">
        {roots(draft).map((kind) => <KindNode key={draft.kinds.indexOf(kind)} app={app} draft={draft} kind={kind} seen={[]} change={change} />)}
        <AddName label="Add a kind" placeholder="kind name" onAdd={(name) => change((next) => { if (!next.kinds.some((kind) => kind.name === name)) next.kinds.push({ name, actions: [], relations: [], parents: [] }); })} />
      </div>
      <SchemaBench app={app} schema={schema} draft={draft} />
    </div>
    <div className="sb-save">
      {version === null ? null : <button type="button" className="btn" disabled={busy || !draft.kinds.length} onClick={() => { void dryRun(); }}>Check the change</button>}
      {version !== null && check ? <CheckAnswer check={check} /> : null}
      <button type="button" className="btn primary" disabled={busy || !draft.kinds.length || (version !== null && (!check || !check.applies))} onClick={() => { void save(); }}>{version === null ? 'Register the app' : 'Save version ' + (version + 1)}</button>
      {refusal ? <p role="alert" className="sb-refused">{refusal}</p> : null}
    </div>
  </section>;
}

/** The kinds drawn at the top of the tree: those with no parent in the draft. */
function roots(draft: Draft): DraftKind[] {
  return draft.kinds.filter((kind) => !kind.parents.some((parent) => draft.kinds.some((other) => other.name === parent)));
}

function KindNode({ app, draft, kind, seen, change }: { app: string; draft: Draft; kind: DraftKind; seen: string[]; change: (edit: (next: Draft) => void) => void }) {
  const at = (next: Draft) => next.kinds.find((entry) => entry.name === kind.name);
  const children = draft.kinds.filter((other) => other.parents[0] === kind.name && !seen.includes(other.name));
  return <div className="sb-node">
    <fieldset className="sb-kind" aria-label={'Kind ' + kind.name}>
      <legend><span className="sb-prefix">{app}.</span><input className="sb-name" aria-label={'Name of kind ' + kind.name} value={kind.name} onChange={(event) => { const name = event.target.value; change((next) => { for (const other of next.kinds) other.parents = other.parents.map((parent) => (parent === kind.name ? name : parent)); const own = at(next); if (own) own.name = name; }); }} /></legend>
      <div className="sb-row"><span className="note">Actions</span>
        {kind.actions.map((action) => <span className="sb-chip" key={action}>{action}<button type="button" aria-label={'Remove action ' + action + ' from ' + kind.name} onClick={() => change((next) => { const own = at(next); if (own) { own.actions = own.actions.filter((held) => held !== action); for (const relation of own.relations) relation.actions = relation.actions.filter((held) => held !== action); } })}>×</button></span>)}
        <AddName label={'Add an action to ' + kind.name} placeholder="action" onAdd={(name) => change((next) => { const own = at(next); if (own && !own.actions.includes(name)) own.actions.push(name); })} />
      </div>
      <table className="sb-relations"><thead><tr><th scope="col">Relation</th>{kind.actions.map((action) => <th scope="col" key={action}>{action}</th>)}<th /></tr></thead>
        <tbody>{kind.relations.map((relation, index) => <tr key={index}>
          <th scope="row"><input className="sb-name" aria-label={'Name of relation ' + relation.name + ' on ' + kind.name} value={relation.name} onChange={(event) => { const name = event.target.value; change((next) => { const own = at(next)?.relations[index]; if (own) own.name = name; }); }} /></th>
          {kind.actions.map((action) => <td key={action}><input type="checkbox" className="sb-tick" aria-label={relation.name + ' carries ' + action} checked={relation.actions.includes(action)} onChange={(event) => { const on = event.target.checked; change((next) => { const own = at(next)?.relations[index]; if (own) own.actions = on ? [...own.actions, action] : own.actions.filter((held) => held !== action); }); }} /></td>)}
          <td><button type="button" className="sb-chip" aria-label={'Remove relation ' + relation.name + ' from ' + kind.name} onClick={() => change((next) => { const own = at(next); if (own) own.relations.splice(index, 1); })}>Remove</button></td>
        </tr>)}</tbody></table>
      <AddName label={'Add a relation to ' + kind.name} placeholder="relation" onAdd={(name) => change((next) => { const own = at(next); if (own && !own.relations.some((relation) => relation.name === name)) own.relations.push({ name, actions: [] }); })} />
      <div className="sb-row"><span className="note">Parents</span>
        {draft.kinds.filter((other) => other.name !== kind.name).map((other) => <label className="sb-chip" key={other.name}><input type="checkbox" className="sb-tick" aria-label={'Parent ' + other.name + ' of ' + kind.name} checked={kind.parents.includes(other.name)} onChange={(event) => { const on = event.target.checked; change((next) => { const own = at(next); if (own) own.parents = on ? [...own.parents, other.name] : own.parents.filter((held) => held !== other.name); }); }} />{other.name}</label>)}
      </div>
      <button type="button" className="sb-chip sb-remove" onClick={() => change((next) => { next.kinds = next.kinds.filter((entry) => entry.name !== kind.name); for (const other of next.kinds) other.parents = other.parents.filter((parent) => parent !== kind.name); })}>Remove kind {kind.name}</button>
    </fieldset>
    {children.length ? <div className="sb-children">{children.map((child) => <KindNode key={draft.kinds.indexOf(child)} app={app} draft={draft} kind={child} seen={[...seen, kind.name]} change={change} />)}</div> : null}
  </div>;
}

/** A name typed and added with its button; nothing is added while it is blank. */
function AddName({ label, placeholder, onAdd }: { label: string; placeholder: string; onAdd: (name: string) => void }) {
  const [name, setName] = useState('');
  const add = () => { const trimmed = name.trim(); if (trimmed) { onAdd(trimmed); setName(''); } };
  return <span className="sb-add"><input className="sb-name" aria-label={label} placeholder={placeholder} value={name} onChange={(event) => setName(event.target.value)} onKeyDown={(event) => { if (event.key === 'Enter') { event.preventDefault(); add(); } }} /><button type="button" className="sb-chip" onClick={add} disabled={!name.trim()}>{label}</button></span>;
}

function CheckAnswer({ check }: { check: SchemaCheck }) {
  const listed = (title: string, items: string[]) => (items.length ? <li>{title}: {items.join(', ')}</li> : null);
  const named = (items: Named[]) => items.map((item) => item.name + ' on ' + item.kind);
  const diff = check.diff;
  return <div className="sb-check" aria-label="What the change would do">
    <ul>
      {listed('Kinds added', diff.kinds_added)}{listed('Kinds removed', diff.kinds_removed)}
      {listed('Relations added', named(diff.relations_added))}{listed('Relations removed', named(diff.relations_removed))}
      {listed('Actions added', named(diff.actions_added))}{listed('Actions removed', named(diff.actions_removed))}
      {listed('Parents added', named(diff.parents_added))}{listed('Parents removed', named(diff.parents_removed))}
    </ul>
    {check.stranded.length
      ? <div role="alert" className="sb-stranded"><b>This change cannot be saved.</b> It would strand standing grants: {check.stranded.map((strand) => strand.count + ' standing grant' + (strand.count === 1 ? '' : 's') + ' of ' + strand.relation + ' on ' + strand.kind).join('; ')}. Revoke them first through the ordinary revoke route.</div>
      : <p className="note">No standing grant is stranded. Saving makes version {check.next}.</p>}
  </div>;
}
