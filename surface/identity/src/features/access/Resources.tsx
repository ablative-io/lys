/** Resources and grant counts judged by the server within the caller's visibility, grouped by kind; a row opens who can reach it. */
import { AccessTabs } from './AccessTabs';
import { request, useLoad } from '../../api';
import { Listing } from '../../shell/Listing';
import type { Column } from '../../shell/Listing';
import { DirectoryGate as Gate } from '../people/Words';
import { clock } from '../file/time';

interface ResourceSummary { kind: string; id: string; standing: number; ended: number; holders: number }
interface ResourcesView { kinds: string[]; resources: ResourceSummary[]; revision: number; judged_at: number }
const readResources = () => request<ResourcesView>('/resources');
const keyOf = (resource: ResourceSummary) => resource.kind + ':' + resource.id;

export function Resources() {
  const load = useLoad(readResources, 'resources');
  return <div className="page fill">
    <Gate load={load} title="Resources" ok={(list) => {
      const groups = list.kinds.map((kind) => {
        const items = list.resources.filter((resource) => resource.kind === kind);
        return { id: kind, name: kind, lead: null, depth: 0, items, within: items };
      }).filter((group) => group.items.length);
      const columns: Column<ResourceSummary>[] = [
        { head: 'Resource', cell: (resource) => resource.id },
        { head: 'Standing grants', cell: (resource) => resource.standing },
        { head: 'Ended grants', cell: (resource) => <span className="sec">{resource.ended}</span> },
        { head: 'Current holders', cell: (resource) => resource.holders },
      ];
      return <>
        <AccessTabs on="resources" />
    <div className="head">
          <div><h1>Resources</h1><p className="sub">Resources named in grants you may see. Counts were checked at {clock(list.judged_at)}, grant revision {list.revision}.</p></div>
        </div>
        <p className="note">Grant counts describe recorded grants. Check access for the permission engine's current decision.</p>
        {list.resources.length ? null : <p>No resources appear in your visible grants yet.</p>}
        <div className="body one">
          <Listing<ResourceSummary> groups={groups} columns={columns} id={keyOf} href={(resource) => '#/access/who/' + encodeURIComponent(keyOf(resource))}
            words={keyOf} noun="resources" holds={(items) => items.length + (items.length === 1 ? ' resource' : ' resources')}
            selected={null} select={() => undefined} />
        </div>
      </>;
    }} />
  </div>;
}
