import { actionWords, resourceFromText, resourceWords } from '../grants/action-words';
/** Read-only permission summaries shared by directory rows and their preview. */
import { api } from '../../api';
import type { Load } from '../../api';
import { resourceText } from '../../generated/grants';
import type { GrantModel, ResourceRef } from '../../generated/grants';
import { reachMap } from '../grants/check';

export interface DirectoryReach { rows: Map<string, Map<string, string[]>>; model: GrantModel }

export async function readDirectoryReach(): Promise<DirectoryReach> {
  const [list, model] = await Promise.all([api.grants(), api.model()]);
  const resources = new Map<string, { resource: ResourceRef; actions: string[] }>();
  for (const grant of list.grants) {
    const key = resourceText(grant.resource);
    const row = resources.get(key) ?? { resource: grant.resource, actions: [] };
    row.actions = [...new Set([...row.actions, ...grant.actions])];
    resources.set(key, row);
  }
  return { rows: await reachMap([...resources.values()]), model };
}

export function Reach({ load, id, compact = false }: { load: Load<DirectoryReach>; id: string; compact?: boolean }) {
  if (load.status === 'loading') return <span className="dim">Reading access…</span>;
  if (load.status === 'refused') return <span className="note" title={load.refused.refusal.reason}>{load.refused.refusal.refusal}: {load.refused.refusal.reason}</span>;
  const rows = [...load.data.rows].filter(([, holders]) => holders.has(id));
  if (compact) return <span>{rows.length} {rows.length === 1 ? 'resource' : 'resources'}</span>;
  if (!rows.length) return <span className="note">No permitted access returned for the resources you may see.</span>;
  return <ul>{rows.map(([resource, holders]) => <li key={resource}>
    <a href={'#/access/who/' + encodeURIComponent(resource)}>{resourceWords(resourceFromText(resource))}</a>
    <span className="note"> · {actionWords(load.data.model, resourceFromText(resource), holders.get(id) ?? [])}</span>
  </li>)}</ul>;
}
