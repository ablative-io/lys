import { AccessTabs } from './AccessTabs';
import { actionWords } from '../grants/action-words';
/** The permission model actually in force, read from the service. */
import { api, useLoad } from '../../api';
import { DirectoryGate as Gate } from '../people/Words';

export function Model() {
  const load = useLoad(api.model, 'permission-model');
  return <div className="page fill">
    <Gate load={load} title="Model" ok={(model) => <>
      <AccessTabs on="model" />
    <div className="head"><div><h1>Model</h1>
        <p className="sub">Permission model version {model.version}. Each relation carries the actions listed below.</p></div>
        <a className="btn" href="#/access">Check access</a></div>
      <div className="pane"><table><thead><tr><th>Relation</th><th>Actions</th></tr></thead><tbody>
        {Object.entries(model.relations).map(([relation, actions]) => <tr key={relation}><td>{relation}</td><td>{actionWords(model, { kind: 'directory', id: 'permissions' }, actions)}</td></tr>)}
      </tbody></table>
      <p className="note">A relation alone grants nothing. Access also requires a current grant, an active identity, and the permission engine's decision.</p></div>
    </>} />
  </div>;
}
