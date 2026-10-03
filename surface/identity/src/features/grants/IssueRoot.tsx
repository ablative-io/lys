/** Root grants are requested explicitly; the server alone decides who may issue one. */
import { useState } from 'react';
import { api, useLoad } from '../../api';
import type { GrantModel, ResourceRef } from '../../generated/grants';
import type { PeopleView } from '../../generated';
import { Gate } from '../signin/Gate';
import { field, RecordedForm } from '../people/RecordedForm';
import { Picker } from '../../shell/Picker';

function Form({ people, model, resources }: { people: PeopleView; model: GrantModel; resources: ResourceRef[] }) {
  const [noExpiry, setNoExpiry] = useState(false);
  const [kind, setKind] = useState('');
  const [result, setResult] = useState(false);
  return <>
    <RecordedForm name="root-grant" title="Issue root grant" done={() => setResult(true)} change={(data) => {
      const relation = field(data, 'relation');
      const actions = model.relations[relation];
      if (!actions) throw new Error('Select a relation from the permission model');
      const expires = noExpiry ? null : Date.parse(field(data, 'expires')) / 1000;
      if (expires !== null && !Number.isFinite(expires)) throw new Error('Enter a valid expiry');
      return { path: '/grants/roots', body: {
        route: 'browser', holder: field(data, 'holder'),
        resource: { kind: field(data, 'kind'), id: field(data, 'resource') }, relation,
        pass_on: data.get('delegate') === 'on' ? { kind: 'to', actions, recipients: ['agent'] } : { kind: 'use_only' },
        window: { starts_at: Math.floor(Date.now() / 1000), ends_at: expires },
      } };
    }}>
      <p>Only the directory's root authority may issue this grant. A root grant is given to a person; that person may delegate only if you permit it below.</p>
      <div className="field">Holder<Picker name="holder" label="Find a person" options={people.people.map((p) => ({ id: p.id, name: p.display_name, detail: p.state }))} /></div>
      <label className="field">Kind of thing<input name="kind" required list="root-kinds" autoComplete="off" onChange={(event) => setKind(event.target.value)} /></label>
      <datalist id="root-kinds">{[...new Set(resources.map((resource) => resource.kind))].sort().map((each) => <option key={each} value={each} />)}</datalist>
      <label className="field">Which one<input name="resource" required list="root-ids" autoComplete="off" /></label>
      <datalist id="root-ids">{resources.filter((resource) => !kind || resource.kind === kind).map((resource) => <option key={resource.kind + ':' + resource.id} value={resource.id} />)}</datalist>
      <label className="field">Relation<select name="relation" required defaultValue=""><option value="" disabled>Choose a relation</option>{Object.entries(model.relations).map(([relation, actions]) => <option key={relation} value={relation}>{relation} · {actions.join(', ')}</option>)}</select></label>
      <label><input name="delegate" type="checkbox" /> Allow this person to pass these actions to their agents</label>
      <div className="field"><label>Expires (your local time)<input name="expires" type="datetime-local" required={!noExpiry} disabled={noExpiry} /></label></div>
      <label><input type="checkbox" checked={noExpiry} onChange={(event) => setNoExpiry(event.target.checked)} /> No expiry of its own</label>
    </RecordedForm>
    {result ? <p><a href="#/access">Close this form and read the grants</a></p> : null}
  </>;
}

async function read() {
  const [people, model] = await Promise.all([api.people(), api.model()]);
  return { people, model };
}

/** The form that issues a root grant, shown in the Access page above the grants. Kinds and names already known are offered; a new one may be typed. */
export function IssueRoot({ resources }: { resources: ResourceRef[] }) {
  const load = useLoad(read, 'root-grant');
  return <Gate load={load} title="Issue access" ok={(data) => <Form {...data} resources={resources} />} />;
}
