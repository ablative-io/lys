/** The permission model actually in force, read from the service, as the one permissions matrix Configuration > Apps shows for every app. */
import { AccessLayout } from './AccessLayout';
import { api, useLoad } from '../../api';
import { DirectoryGate as Gate } from '../people/Words';
import { SchemaMatrix } from '../apps/SchemaMatrix';

export function Model() {
  const load = useLoad(api.model, 'permission-model');
  return <AccessLayout on="model" subtitle={(load.status === 'ok' ? 'Permission model version ' + load.data.version + '. ' : '') + 'What each relation may do. A relation alone grants nothing: access also needs a current grant, an active identity, and the permission engine’s decision.'}>
    <Gate load={load} title="Model" ok={(model) => <div className="pane">
      <SchemaMatrix kinds={[['What it may do', { relations: model.relations }]]} label="Permission model" sentence={(action) => model.action_sentences[action]} />
    </div>} />
  </AccessLayout>;
}
