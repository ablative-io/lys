/** Root grants are requested explicitly; the server alone decides who may issue one. */
import { useState } from 'react';
import { api, useLoad } from '../../api';
import type { GrantModel, ResourceRef } from '../../generated/grants';
import type { PeopleView } from '../../generated';
import { Gate } from '../signin/Gate';
import { field, RecordedForm } from '../people/RecordedForm';
import { Picker } from '../../shell/Picker';

/** One row you put things into: holder, kind of thing, which one, relation, may pass on, expiry, Issue. */
function Form({ people, model, resources, done }: { people: PeopleView; model: GrantModel; resources: ResourceRef[]; done: () => void }) {
  const [noExpiry, setNoExpiry] = useState(false);
  const [kind, setKind] = useState('');
  return <RecordedForm name="root-grant" title="Issue root grant" submitLabel="Issue" drawn={{ symbol: 'add', word: 'Issue' }} done={done} change={(data) => {
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
    <p className="note">Only the directory's root authority may issue this grant. It is given to a person, who may pass it to their agents only if you allow it here.</p>
    <div className="field">Holder<Picker name="holder" label="Find a person" options={people.people.map((p) => ({ id: p.id, name: p.display_name, detail: p.state }))} /></div>
    <label className="field">Kind of thing<input name="kind" required list="root-kinds" autoComplete="off" onChange={(event) => setKind(event.target.value)} /></label>
    <datalist id="root-kinds">{[...new Set(resources.map((resource) => resource.kind))].sort().map((each) => <option key={each} value={each} />)}</datalist>
    <label className="field">Which one<input name="resource" required list="root-ids" autoComplete="off" /></label>
    <datalist id="root-ids">{resources.filter((resource) => !kind || resource.kind === kind).map((resource) => <option key={resource.kind + ':' + resource.id} value={resource.id} />)}</datalist>
    <label className="field">Relation<select name="relation" required defaultValue=""><option value="" disabled>Choose a relation</option>{Object.entries(model.relations).map(([relation, actions]) => <option key={relation} value={relation}>{relation} · {actions.join(', ')}</option>)}</select></label>
    <div className="field">May pass on<label className="tick"><input name="delegate" type="checkbox" /> to their agents</label></div>
    <div className="field">Expires (your local time)<div className="expires"><input name="expires" type="datetime-local" aria-label="Expires" required={!noExpiry} disabled={noExpiry} />
      <label className="tick"><input type="checkbox" checked={noExpiry} onChange={(event) => setNoExpiry(event.target.checked)} /> no expiry</label></div></div>
  </RecordedForm>;
}

async function read() {
  const [people, model] = await Promise.all([api.people(), api.model()]);
  return { people, model };
}

/** The form that issues a root grant, shown in the Access page above the grants. Kinds and names already known are offered; a new one may be typed. */
export function IssueRoot({ resources, done = () => undefined }: { resources: ResourceRef[]; done?: () => void }) {
  const load = useLoad(read, 'root-grant');
  return <Gate load={load} title="Issue access" ok={(data) => <Form {...data} resources={resources} done={done} />} />;
}
