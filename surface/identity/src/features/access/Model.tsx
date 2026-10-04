/** The permission model actually in force, read from the service, as the one permissions matrix Configuration > Apps shows for every app. */
import { AccessTabs } from './AccessTabs';
import { api, useLoad } from '../../api';
import { DirectoryGate as Gate } from '../people/Words';
import { SchemaMatrix } from '../apps/SchemaMatrix';

export function Model() {
  const load = useLoad(api.model, 'permission-model');
  return <div className="page fill">
    <div className="head"><div><h1>Access</h1>
      <p className="sub">{load.status === 'ok' ? 'Permission model version ' + load.data.version + '. ' : ''}What each relation may do. A relation alone grants nothing: access also needs a current grant, an active identity, and the permission engine’s decision.</p></div></div>
    <AccessTabs on="model" />
    <Gate load={load} title="Model" ok={(model) => <div className="pane">
      <SchemaMatrix kinds={[['What it may do', { relations: model.relations }]]} label="Permission model" sentence={(action) => model.action_sentences[action]} />
    </div>} />
  </div>;
}
