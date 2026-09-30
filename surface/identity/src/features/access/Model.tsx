/** The permission model actually in force, read from the service. */
import { api, useLoad } from '../../api';
import { DirectoryGate as Gate } from '../people/Words';

export function Model() {
  const load = useLoad(api.model, 'permission-model');
  return <div className="page"><div className="eyebrow">Access</div><h1>Model</h1>
    <Gate load={load} title="Model" ok={(model) => <>
      <p className="sub">Permission model version {model.version}. Each relation carries the actions listed below.</p>
      <table><thead><tr><th>Relation</th><th>Actions</th></tr></thead><tbody>
        {Object.entries(model.relations).map(([relation, actions]) => <tr key={relation}><td>{relation}</td><td>{actions.join(', ')}</td></tr>)}
      </tbody></table>
      <p>A relation alone grants nothing. Access also requires a current grant, an active identity, and the permission engine's decision.</p>
      <a className="btn" href="#/access">Check access</a>
    </>} />
  </div>;
}
