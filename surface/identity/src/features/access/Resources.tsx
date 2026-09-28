/** Resources named by the grants the caller may see, never an invented global registry. */
import { api, useLoad } from '../../api';
import { resourceText } from '../../generated/grants';
import { Gate } from '../signin/Gate';

export function Resources() {
  const load = useLoad(api.grants, 'grant-resources');
  return <div className="page"><div className="eyebrow">Access</div><h1>Resources</h1>
    <Gate load={load} title="Resources" ok={(list) => {
      const resources = new Map(list.grants.map((grant) => [resourceText(grant.resource), grant.resource]));
      return <>
        <p className="sub">Resources named in the grants you may see, at grant revision {list.revision}. This includes historical grants; check access for a current decision.</p>
        <table><thead><tr><th>Kind</th><th>Resource</th><th>Recorded grants</th><th>Access</th></tr></thead><tbody>
          {[...resources].map(([key, resource]) => <tr key={key}>
            <td>{resource.kind}</td><td>{resource.id}</td>
            <td>{list.grants.filter((grant) => resourceText(grant.resource) === key).length}</td>
            <td><a href={'#/access/who/' + encodeURIComponent(key)}>Who can reach this?</a></td>
          </tr>)}
        </tbody></table>
        {resources.size === 0 ? <p>No resources appear in your visible grants yet.</p> : null}
        <a className="btn" href="#/access/issue">Issue root grant</a>
      </>;
    }} />
  </div>;
}
