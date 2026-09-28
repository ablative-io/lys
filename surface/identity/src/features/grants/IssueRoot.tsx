/** Root grants are requested explicitly; the server alone decides who may issue one. */
import { useState } from 'react';
import { api, useLoad } from '../../api';
import type { GrantModel } from '../../generated/grants';
import type { PeopleView } from '../../generated';
import { Gate } from '../signin/Gate';
import { field, RecordedForm, TextField } from '../people/RecordedForm';

function Form({ people, model }: { people: PeopleView; model: GrantModel }) {
  const [noExpiry, setNoExpiry] = useState(false);
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
      <label className="field">Holder<select name="holder" required defaultValue=""><option value="" disabled>Choose a person</option>{people.people.map((p) => <option key={p.id} value={p.id}>{p.display_name} · {p.state}</option>)}</select></label>
      <TextField name="kind" label="Resource kind" />
      <TextField name="resource" label="Resource ID" />
      <label className="field">Relation<select name="relation" required defaultValue=""><option value="" disabled>Choose a relation</option>{Object.entries(model.relations).map(([relation, actions]) => <option key={relation} value={relation}>{relation} · {actions.join(', ')}</option>)}</select></label>
      <label><input name="delegate" type="checkbox" /> Allow this person to pass these actions to their agents</label>
      <div className="field"><label>Expires (your local time)<input name="expires" type="datetime-local" required={!noExpiry} disabled={noExpiry} /></label></div>
      <label><input type="checkbox" checked={noExpiry} onChange={(event) => setNoExpiry(event.target.checked)} /> No expiry of its own</label>
    </RecordedForm>
    {result ? <p><a href="#/access">View recorded grants and check access</a></p> : null}
  </>;
}

async function read() {
  const [people, model] = await Promise.all([api.people(), api.model()]);
  return { people, model };
}

export function IssueRoot() {
  const load = useLoad(read, 'root-grant');
  return <div className="page"><a href="#/access">Access</a><h1>Issue access</h1><Gate load={load} title="Issue access" ok={(data) => <Form {...data} />} /></div>;
}
