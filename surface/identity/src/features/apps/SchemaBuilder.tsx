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
import { operationId, send } from '../../api';
import { SchemaBench } from './SchemaBench';
import { AddName, KindNode } from './kind-builder';
import './schema-builder.css';
import { Act } from '../../shell/Act';
import { ErrorWords } from '../people/Words';

/** A kind as the schema JSON writes it. */
export interface SchemaKindJson {
  actions: string[];
  relations?: Record<string, string[]>;
  parents?: string[];
  hot?: string[];
  agents?: string[];
  roles?: Record<string, string[]>;
}
/** An app's schema as the API takes it. */
export interface SchemaJson {
  kinds: Record<string, SchemaKindJson>;
}
/** One relation of a draft kind. */
export interface DraftRelation {
  name: string;
  actions: string[];
}
/** One kind of the draft, with omitted optional members remembered so opening and saving does not add them. */
export interface DraftKind {
  name: string;
  actions: string[];
  relations: DraftRelation[];
  parents: string[];
  hot?: string[];
  agents?: string[];
  roles?: Record<string, string[]>;
  omitted?: ('relations' | 'parents')[];
}
/** The draft being built. */
export interface Draft {
  kinds: DraftKind[];
}

/** What a change would do, as the dry run answers it. */
export interface Named {
  kind: string;
  name: string;
}
export interface Strand {
  kind: string;
  relation: string;
  count: number;
}
export interface SchemaCheck {
  current: number;
  next: number;
  applies: boolean;
  stranded: Strand[];
  diff: {
    kinds_added: string[];
    kinds_removed: string[];
    relations_added: Named[];
    relations_removed: Named[];
    actions_added: Named[];
    actions_removed: Named[];
    parents_added: Named[];
    parents_removed: Named[];
    agents_added?: Named[];
    agents_removed?: Named[];
    roles_added?: Named[];
    roles_removed?: Named[];
    roles_changed?: { kind: string; role: string; words: string[] }[];
  };
}

export { send };

/** The schema JSON the draft makes, every kind under `app`'s prefix. */
export function toSchema(app: string, draft: Draft): SchemaJson {
  const kinds: Record<string, SchemaKindJson> = {};
  for (const kind of draft.kinds) {
    const body: SchemaKindJson = { actions: [...kind.actions] };
    if (!kind.omitted?.includes('relations') || kind.relations.length) body.relations = Object.fromEntries(kind.relations.map((relation) => [relation.name, relation.actions.filter((action) => kind.actions.includes(action))]));
    if (!kind.omitted?.includes('parents') || kind.parents.length) body.parents = kind.parents.map((parent) => app + '.' + parent);
    // A hot action is kept only while its kind still declares it; none left, and `hot` is not written.
    const hot = (kind.hot ?? []).filter((action) => kind.actions.includes(action));
    if (hot.length || kind.hot?.length === 0) body.hot = hot;
    // Removing a declared action removes it from agent access too, without widening what agents may hold.
    if (kind.agents) body.agents = kind.agents.filter((action) => kind.actions.includes(action));
    // A role keeps only the actions its kind still declares; one left carrying none is dropped, and the dry run names it.
    const roles = Object.entries(kind.roles ?? {}).flatMap(([role, may]) => {
      const kept = may.filter((action) => kind.actions.includes(action));
      return kept.length || !may.length ? [[role, kept] as const] : [];
    });
    if (roles.length || (kind.roles && !Object.keys(kind.roles).length)) body.roles = Object.fromEntries(roles);
    kinds[app + '.' + kind.name] = body;
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
      ...(kind.hot ? { hot: [...kind.hot] } : {}),
      ...(kind.agents ? { agents: [...kind.agents] } : {}),
      ...(kind.roles ? { roles: Object.fromEntries(Object.entries(kind.roles).map(([role, actions]) => [role, [...actions]])) } : {}),
      omitted: (['relations', 'parents'] as const).filter((member) => !Object.hasOwn(kind, member)),
    })),
  };
}

/** The templates a draft starts from. */
export const TEMPLATES: { id: string; label: string; draft: Draft }[] = [
  {
    id: 'owner-editor-viewer',
    label: 'Owner, editor and viewer on one kind',
    draft: {
      kinds: [
        {
          name: 'document',
          actions: ['read', 'write', 'share'],
          relations: [
            { name: 'owner', actions: ['read', 'write', 'share'] },
            { name: 'editor', actions: ['read', 'write'] },
            { name: 'viewer', actions: ['read'] },
          ],
          parents: [],
        },
      ],
    },
  },
  {
    id: 'workspace',
    label: 'A workspace whose members reach its channels',
    draft: {
      kinds: [
        {
          name: 'workspace',
          actions: ['read', 'write', 'admin'],
          relations: [
            { name: 'owner', actions: ['read', 'write', 'admin'] },
            { name: 'member', actions: ['read'] },
          ],
          parents: [],
        },
        { name: 'channel', actions: ['read', 'write'], relations: [{ name: 'poster', actions: ['read', 'write'] }], parents: ['workspace'] },
      ],
    },
  },
  {
    id: 'team-scoped',
    label: 'Resources scoped to a team',
    draft: {
      kinds: [
        {
          name: 'team',
          actions: ['read', 'manage'],
          relations: [
            { name: 'lead', actions: ['read', 'manage'] },
            { name: 'member', actions: ['read'] },
          ],
          parents: [],
        },
        { name: 'project', actions: ['read', 'write'], relations: [{ name: 'contributor', actions: ['read', 'write'] }], parents: ['team'] },
      ],
    },
  },
];

const copy = (draft: Draft): Draft => JSON.parse(JSON.stringify(draft)) as Draft;
const storageKey = (app: string) => 'lys.schema-draft.' + app;

function kept(app: string, initial: Draft): Draft {
  const stored = sessionStorage.getItem(storageKey(app));
  if (!stored) return initial;
  try {
    const draft = JSON.parse(stored) as Draft;
    // Older drafts could not edit agent access and never stored it; carry it from the uploaded kind.
    for (const kind of draft.kinds) {
      const uploaded = initial.kinds.find((entry) => entry.name === kind.name);
      if (kind.agents === undefined && uploaded?.agents) kind.agents = [...uploaded.agents];
    }
    return draft;
  } catch {
    return initial;
  }
}

/** The builder for `app`'s schema at `version`, opened from `initial`. */
export function SchemaBuilder({ app, version, initial, onSaved, register }: { app: string; version: number | null; initial?: SchemaJson; onSaved?: (message: string) => void; register?: (schema: SchemaJson) => Promise<void> }) {
  const [opened] = useState(() => {
    try {
      return { draft: initial ? fromSchema(app, initial) : { kinds: [] }, error: '' };
    } catch (error) {
      return { draft: { kinds: [] }, error: String(error) };
    }
  });
  const [draft, setDraft] = useState<Draft>(() => kept(app, opened.draft));
  const [check, setCheck] = useState<SchemaCheck | null>(null);
  // What failed: a problem said through ErrorWords, its code kept under Error details.
  const [refusal, setRefusal] = useState<unknown>(opened.error ? new Error(opened.error.replace(/^Error:\s*/, '')) : null);
  const [busy, setBusy] = useState(false);
  useEffect(() => {
    sessionStorage.setItem(storageKey(app), JSON.stringify(draft));
  }, [app, draft]);
  const change = (edit: (next: Draft) => void) => {
    const next = copy(draft);
    edit(next);
    setDraft(next);
    setCheck(null);
  };
  const schema = toSchema(app, draft);
  const refused = (error: unknown) => setRefusal(error);
  const dryRun = async () => {
    setBusy(true);
    setRefusal(null);
    try {
      setCheck(await send<SchemaCheck>('POST', '/apps/' + encodeURIComponent(app) + '/schema/check', { replaces: version, schema }));
    } catch (error) {
      refused(error);
    }
    setBusy(false);
  };
  const save = async () => {
    setBusy(true);
    setRefusal(null);
    try {
      if (version === null) {
        await register?.(schema);
      } else {
        await send('PUT', '/apps/' + encodeURIComponent(app) + '/schema', { operation: operationId(), replaces: version, schema });
      }
      sessionStorage.removeItem(storageKey(app));
      setCheck(null);
      onSaved?.(version === null ? 'The app is registered and waits for approval.' : 'The schema was saved.');
    } catch (error) {
      refused(error);
    }
    setBusy(false);
  };
  // A greyed act always says why, beside it.
  const saveWhyNot = busy
    ? ''
    : !draft.kinds.length
      ? 'Add a kind, or start from a template, to build the permissions.'
      : version !== null && !check
        ? 'Check the change first: Lys shows what it adds and removes before it can be saved.'
        : version !== null && check && !check.applies
          ? 'This change cannot be saved as it is; the check above says why.'
          : '';
  return (
    <section className="sb" aria-label={'Permission template for ' + app}>
      <div className="sb-templates" role="group" aria-label="Start from a template">
        <span className="note">Start from</span>
        {TEMPLATES.map((template) => (
          <button
            type="button"
            className="sb-chip"
            data-choice=""
            key={template.id}
            onClick={() =>
              change((next) => {
                next.kinds = copy(template.draft).kinds;
              })
            }
          >
            {template.label}
          </button>
        ))}
      </div>
      <div className="sb-layout">
        <div className="sb-tree" aria-label="Kinds">
          {roots(draft).map((kind) => (
            <KindNode key={draft.kinds.indexOf(kind)} app={app} draft={draft} kind={kind} seen={[]} change={change} />
          ))}
          <AddName
            label="Add a kind"
            placeholder="kind name"
            onAdd={(name) =>
              change((next) => {
                if (!next.kinds.some((kind) => kind.name === name)) next.kinds.push({ name, actions: [], relations: [], parents: [] });
              })
            }
          />
        </div>
        <SchemaBench app={app} schema={schema} draft={draft} />
      </div>
      <div className="sb-save">
        {version === null ? null : (
          <Act
            symbol="again"
            name="Check the change"
            word="Check"
            disabled={busy || !draft.kinds.length}
            onClick={() => {
              void dryRun();
            }}
          />
        )}
        {version !== null && check ? <CheckAnswer check={check} /> : null}
        <Act
          symbol={version === null ? 'add' : 'save'}
          name={version === null ? 'Register the app' : 'Save version ' + (version + 1)}
          word={version === null ? 'Register' : 'Save'}
          tone="primary"
          disabled={busy || !draft.kinds.length || (version !== null && (!check || !check.applies))}
          onClick={() => {
            void save();
          }}
        />
        {saveWhyNot ? <p className="hint sb-why-not">{saveWhyNot}</p> : null}
        {refusal ? (
          <div className="sb-refused">
            <ErrorWords problem={refusal} />
          </div>
        ) : null}
      </div>
    </section>
  );
}

/** The kinds drawn at the top of the tree: those with no parent in the draft. */
function roots(draft: Draft): DraftKind[] {
  return draft.kinds.filter((kind) => !kind.parents.some((parent) => draft.kinds.some((other) => other.name === parent)));
}

function CheckAnswer({ check }: { check: SchemaCheck }) {
  const listed = (title: string, items: string[]) =>
    items.length ? (
      <li>
        {title}: {items.join(', ')}
      </li>
    ) : null;
  const named = (items: Named[]) => items.map((item) => item.name + ' on ' + item.kind);
  const diff = check.diff;
  return (
    <div className="sb-check" aria-label="What the change would do">
      <ul>
        {listed('Kinds added', diff.kinds_added)}
        {listed('Kinds removed', diff.kinds_removed)}
        {listed('Relations added', named(diff.relations_added))}
        {listed('Relations removed', named(diff.relations_removed))}
        {listed('Actions added', named(diff.actions_added))}
        {listed('Actions removed', named(diff.actions_removed))}
        {listed('Agent actions added', named(diff.agents_added ?? []))}
        {listed('Agent actions removed', named(diff.agents_removed ?? []))}
        {listed('Parents added', named(diff.parents_added))}
        {listed('Parents removed', named(diff.parents_removed))}
        {listed('Roles added', named(diff.roles_added ?? []))}
        {listed('Roles removed', named(diff.roles_removed ?? []))}
        {listed(
          'Roles changed',
          (diff.roles_changed ?? []).flatMap((change) => change.words)
        )}
      </ul>
      {check.stranded.length ? (
        <div role="alert" className="sb-stranded">
          <b>This change cannot be saved.</b> It would strand standing grants: {check.stranded.map((strand) => strand.count + ' standing grant' + (strand.count === 1 ? '' : 's') + ' of ' + strand.relation + ' on ' + strand.kind).join('; ')}. Take them back on Access first, then check again.
        </div>
      ) : (
        <p className="note">No standing grant is stranded. Saving makes version {check.next}.</p>
      )}
    </div>
  );
}
