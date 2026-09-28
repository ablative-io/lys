/** Directory actions follow the same page header and tab navigation as identity files. */
import { useState } from 'react';
import { useSearchParams } from 'react-router';
import { api, useLoad } from '../../api';
import type { PeopleView } from '../../generated';
import { Gate } from '../signin/Gate';
import { ACTIONS } from '../file/tabs';
import { field, RecordedForm, TextField } from './RecordedForm';
import './manage.css';

const SECTIONS = [
  ['person', 'Register a person'],
  ['agent', 'Register an agent'],
  ['login', 'Connect sign-in'],
  ['status', 'Change status'],
  ['profile', 'Edit name'],
] as const;

function Forms({ initial }: { initial: PeopleView }) {
  const [params] = useSearchParams();
  const section = SECTIONS.find(([key]) => key === params.get('action'))?.[0] ?? 'person';
  const [people, setPeople] = useState(initial);
  const [refreshError, setRefreshError] = useState('');
  const reload = () => {
    api.people().then((value) => { setPeople(value); setRefreshError(''); }, (error: unknown) => setRefreshError(String(error)));
  };
  const identities = people.people.flatMap((p) => [p, ...p.agents]);
  const [selected, setSelected] = useState(() => identities.find((entry) => entry.id === params.get('identity'))?.id ?? '');
  const identity = identities.find((p) => p.id === selected);
  if (people.scope !== 'directory') return <p className="why-not">Only the configured directory administrator may register identities or change their records.</p>;
  return <>
    <nav className="tabs" aria-label="Directory actions">
      {SECTIONS.map(([key, title]) => <a key={key} href={'#/directory/manage?action=' + key + (params.get('advanced') === '1' ? '&advanced=1' : '')} className={section === key ? 'on' : ''} aria-current={section === key ? 'page' : undefined}>{title}</a>)}
    </nav>
    {refreshError ? <p className="why-not" role="alert">The change was recorded but the directory refresh failed: {refreshError}</p> : null}
    <section className="directory-editor">
      {section === 'person' ? <RecordedForm name="register-person" title="Register a person" description="Add someone to the directory. You can activate their identity and give them access separately." done={reload} change={(data) => ({ path: '/people', body: { display_name: field(data, 'display_name') } })}>
        <TextField name="display_name" label="Full name" />
        <p className="note">Usually their first name and surname. A single name is fine too.</p>
      </RecordedForm> : null}
      {section === 'agent' ? <RecordedForm name="register-agent" title="Register an agent" description="Add an agent that answers to you. Registration does not activate it or give it access." done={reload} change={(data) => ({ path: '/agents', body: { display_name: field(data, 'display_name') } })}>
        <TextField name="display_name" label="Agent's name" />
      </RecordedForm> : null}
      {section === 'login' ? <RecordedForm name="bind-login" title="Bind a sign-in identity" heading="Connect sign-in" description="Connect a person's directory record to their account with a sign-in provider." submitLabel="Connect sign-in" done={reload} change={(data) => ({ path: '/people/' + encodeURIComponent(field(data, 'person')) + '/logins', body: { issuer: field(data, 'issuer'), subject: field(data, 'subject') } })}>
        <label className="field">Person<select name="person" required defaultValue=""><option value="" disabled>Choose a person</option>{people.people.map((p) => <option key={p.id} value={p.id}>{p.display_name} · {p.id}</option>)}</select></label>
        <TextField name="issuer" label="Issuer URL" />
        <TextField name="subject" label="Account subject" />
        <p className="note">Use the provider's exact subject identifier, not the person's name or email address.</p>
        {people.people.length === 0 ? <p className="note">Register a person before connecting their sign-in.</p> : null}
      </RecordedForm> : null}
      {section === 'profile' ? <RecordedForm name="change-profile" title="Save name" heading="Edit name" description="Change the display name while keeping the same identity and audit history." done={reload} change={(data) => ({ path: '/identities/' + encodeURIComponent(field(data, 'identity')) + '/profile', body: { display_name: field(data, 'display_name') } })}>
        <label className="field">Identity<select name="identity" required value={selected} onChange={(event) => setSelected(event.target.value)}><option value="" disabled>Choose an identity</option>{identities.map((p) => <option key={p.id} value={p.id}>{p.display_name}</option>)}</select></label>
        <label className="field">Display name<input name="display_name" required key={selected} defaultValue={identity?.display_name ?? ''} /></label>
      </RecordedForm> : null}
      {section === 'status' ? <RecordedForm name="lifecycle" title="Record lifecycle change" heading="Change status" description="Activate, suspend or retire a person or agent. Only changes available for their current status are shown." submitLabel="Record change" done={reload} change={(data) => ({ path: '/identities/' + encodeURIComponent(field(data, 'identity')) + '/transitions', body: { transition: field(data, 'transition'), reason: field(data, 'reason') } })}>
        <label className="field">Identity<select name="identity" required value={selected} onChange={(e) => setSelected(e.target.value)}><option value="" disabled>Choose an identity</option>{identities.map((p) => <option key={p.id} value={p.id}>{p.display_name} · {p.state}</option>)}</select></label>
        <label className="field">Change<select name="transition" required key={identity?.state} defaultValue=""><option value="" disabled>Choose a change</option>{(identity ? ACTIONS[identity.state] : []).map((action) => <option key={action} value={action}>{action}</option>)}</select></label>
        <TextField name="reason" label="Reason" />
        <p className="note">Suspending or retiring an identity stops its access. Retirement is permanent.</p>
      </RecordedForm> : null}
    </section>
  </>;
}

export function Manage() {
  const load = useLoad(api.people, 'manage');
  return <div className="page directory-manage">
    <div className="head">
      <div><div className="eyebrow">Directory</div><h1>Manage directory</h1><p className="sub">Register people and agents, connect sign-in accounts, and manage their status.</p></div>
      <a className="btn" href="#/people">Back to people</a>
    </div>
    <Gate load={load} title="Manage directory" ok={(people) => <Forms initial={people} />} />
  </div>;
}
