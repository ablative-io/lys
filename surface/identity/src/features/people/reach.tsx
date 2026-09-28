/** Read-only permission summaries shared by directory rows and their preview. */
import { api } from '../../api';
import type { Load } from '../../api';
import { resourceText } from '../../generated/grants';
import type { ResourceRef } from '../../generated/grants';
import { reachMap } from '../grants/check';

export type DirectoryReach = Map<string, Map<string, string[]>>;

export async function readDirectoryReach(): Promise<DirectoryReach> {
  const list = await api.grants();
  const resources = new Map<string, { resource: ResourceRef; actions: string[] }>();
  for (const grant of list.grants) {
    const key = resourceText(grant.resource);
    const row = resources.get(key) ?? { resource: grant.resource, actions: [] };
    row.actions = [...new Set([...row.actions, ...grant.actions])];
    resources.set(key, row);
  }
  return reachMap([...resources.values()]);
}

export function Reach({ load, id, compact = false }: { load: Load<DirectoryReach>; id: string; compact?: boolean }) {
  if (load.status === 'loading') return <span className="dim">Reading access…</span>;
  if (load.status === 'refused') return <span className="note" title={load.refused.refusal.reason}>{load.refused.refusal.refusal}: {load.refused.refusal.reason}</span>;
  const rows = [...load.data].filter(([, holders]) => holders.has(id));
  if (compact) return <span>{rows.length} {rows.length === 1 ? 'resource' : 'resources'}</span>;
  if (!rows.length) return <span className="note">No permitted access returned for the resources you may see.</span>;
  return <ul>{rows.map(([resource, holders]) => <li key={resource}>
    <a href={'#/access/who/' + encodeURIComponent(resource)}>{resource}</a>
    <span className="note"> · {holders.get(id)?.join(', ')}</span>
  </li>)}</ul>;
}
