/** Resources and grant counts judged by the server within the caller's visibility, grouped by kind; a row opens who can reach it. */
import { AccessTabs } from './AccessTabs';
import { api, request, useLoad } from '../../api';
import { entries } from '../people/directory';
import { Listing } from '../../shell/Listing';
import type { Column } from '../../shell/Listing';
import { DirectoryGate as Gate } from '../people/Words';
import { clock } from '../file/time';
import { counted } from '../../shell/count';

interface ResourceSummary { kind: string; id: string; standing: number; ended: number; holders: number }
interface ResourcesView { kinds: string[]; resources: ResourceSummary[]; revision: number; judged_at: number }
const keyOf = (resource: ResourceSummary) => resource.kind + ':' + resource.id;
/** A resource that is itself a person or an agent is read by its name, never its raw id. */
const identity = (id: string) => /^(?:person|agent)-[0-9a-f]{32}$/.test(id);
/** The resources, and the names of the people and agents among them; the directory is read only when one of them is an identity. */
async function readResources(): Promise<{ list: ResourcesView; label: (resource: ResourceSummary) => string }> {
  const list = await request<ResourcesView>('/resources');
  const names = list.resources.some((resource) => identity(resource.id))
    ? new Map(entries(await api.people()).map((entry) => [entry.id, entry.display_name]))
    : new Map<string, string>();
  return { list, label: (resource) => identity(resource.id) ? names.get(resource.id) ?? 'someone outside your view' : resource.id };
}

export function Resources() {
  const load = useLoad(readResources, 'resources');
  return <div className="page fill">
    <Gate load={load} title="Resources" ok={({ list, label }) => {
      const groups = list.kinds.map((kind) => {
        const items = list.resources.filter((resource) => resource.kind === kind);
        return { id: kind, name: kind, lead: null, depth: 0, items, within: items };
      }).filter((group) => group.items.length);
      const columns: Column<ResourceSummary>[] = [
        { head: 'Resource', cell: (resource) => <a href={'#/access/who/' + encodeURIComponent(keyOf(resource))}>{label(resource)}</a> },
        { head: 'Standing grants', cell: (resource) => resource.standing },
        { head: 'Ended grants', cell: (resource) => <span className="sec">{resource.ended}</span> },
        { head: 'Current holders', cell: (resource) => resource.holders },
      ];
      return <>
        <div className="head">
          <div><h1>Access</h1><p className="sub">Resources named in grants you may see. Counts were checked at {clock(list.judged_at)}, grant revision {list.revision}; they describe recorded grants. Ask for the permission engine's current decision.</p></div>
        </div>
        <AccessTabs on="resources" />
        <div className="body one">
          <Listing<ResourceSummary> groups={groups} columns={columns} id={keyOf} href={(resource) => '#/access/who/' + encodeURIComponent(keyOf(resource))}
            words={(resource) => resource.kind + ' ' + label(resource)} noun="resources" empty="No resources appear in your visible grants yet." holds={(items) => counted(items.length, 'resources')}
            selected={null} select={() => undefined} />
        </div>
      </>;
    }} />
  </div>;
}
