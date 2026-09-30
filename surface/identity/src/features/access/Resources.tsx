/** Resources and grant counts judged by the server within the caller's visibility. */
import { useState } from 'react';
import { request, useLoad } from '../../api';
import { DirectoryGate as Gate } from '../people/Words';

interface ResourceSummary { kind: string; id: string; standing: number; ended: number; holders: number }
interface ResourcesView { kinds: string[]; resources: ResourceSummary[]; revision: number; judged_at: number }
const readResources = () => request<ResourcesView>('/resources');

export function Resources() {
  const [kind, setKind] = useState('');
  const load = useLoad(readResources, 'resources');
  return <div className="page"><div className="eyebrow">Access</div><h1>Resources</h1>
    <Gate load={load} title="Resources" ok={(list) => <>
      <p className="sub">Resources named in grants you may see. Counts were checked at {new Date(list.judged_at * 1000).toLocaleString('en-AU', { timeZone: 'Australia/Melbourne' })} Melbourne time, grant revision {list.revision}.</p>
      <label>Resource kind <select value={kind} onChange={(event) => setKind(event.target.value)}>
        <option value="">All kinds</option>{list.kinds.map((value) => <option key={value} value={value}>{value}</option>)}
      </select></label>
      <table><thead><tr><th>Kind</th><th>Resource</th><th>Standing grants</th><th>Ended grants</th><th>Current holders</th><th>Access</th></tr></thead><tbody>
        {list.resources.filter((resource) => !kind || resource.kind === kind).map((resource) => {
          const key = resource.kind + ':' + resource.id;
          return <tr key={key}><td>{resource.kind}</td><td>{resource.id}</td><td>{resource.standing}</td><td>{resource.ended}</td><td>{resource.holders}</td>
            <td><a href={'#/access/who/' + encodeURIComponent(key)}>Who can reach this?</a></td></tr>;
        })}
      </tbody></table>
      {list.resources.length === 0 ? <p>No resources appear in your visible grants yet.</p> : null}
      <p className="note">Grant counts describe recorded grants. Check access for the permission engine's current decision.</p>
      <a className="btn" href="#/access/issue">Issue root grant</a>
    </>} />
  </div>;
}
