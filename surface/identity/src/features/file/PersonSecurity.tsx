/** A person's credentials are sign-in bindings; their sessions are browser sign-ins, never agent runtimes. */
import { useState } from 'react';
import { api, operationId, request, useLoad } from '../../api';
import { SessionList } from '../sessions/Sessions';
import { DirectoryGate as Gate } from '../people/Words';
import { RecordedForm } from '../people/RecordedForm';
import { AdmissionWaiting, PersonAccount } from '../people/Account';
import { reasonOf } from '../sign-in/SignIn';
import { Act } from '../../shell/Act';
import type { PersonSummary } from '../../generated';

export function PersonCredentials({ id }: { id: string }) {
  const [revision, setRevision] = useState(0);
  const [moveNotice, setMoveNotice] = useState('');
  const load = useLoad(async () => {
    const me = await api.me();
    const logins = me.person.id === id ? me.sign_in_identities.map((login) => ({ issuer: login.provider, subject: login.subject })) : (await api.identity(id)).logins;
    const people = await api.people();
    return { logins, own: me.person.id === id, directory: people.scope === 'directory', people: people.people };
  }, 'person-credentials:' + id + ':' + revision);
  return <section className="card"><h2>Sign-in accounts</h2><p>These accounts sign this person into Lys. Passwords and secret values are never shown here.</p>
    {moveNotice ? <p role="status">{moveNotice}</p> : null}
    <Gate load={load} title="Sign-in accounts" ok={({ logins, own, directory, people }) => <>
      {logins.length ? <table><thead><tr><th>Sign-in provider</th><th>Account identifier</th></tr></thead><tbody>{logins.map((login) => <tr key={JSON.stringify([login.issuer, login.subject])}><td>{login.issuer}</td><td>{login.subject}{directory ? <MoveLogin id={id} login={login} people={people} moved={(words) => { setMoveNotice(words); setRevision((value) => value + 1); }} /> : null}</td></tr>)}</tbody></table> : <p>No sign-in account is bound to this person.</p>}
      {own ? <p className="note">Your own email and password are in <a href="#/settings/account">Settings › Your account</a>.</p> : null}
      {directory ? <>
        <AdmissionWaiting id={id} />
        <RecordedForm name="bind-login" title="Bind a sign-in identity" heading="Connect a sign-in account" description="Connect this person's directory record to their account with a sign-in provider." submitLabel="Connect sign-in" drawn={{ symbol: 'add', word: 'Connect' }} done={() => setRevision((value) => value + 1)}
          change={(form) => ({ path: '/people/' + encodeURIComponent(id) + '/logins', body: { issuer: String(form.get('issuer') ?? '').trim(), subject: String(form.get('subject') ?? '').trim() } })}>
          <div className="form-grid">
            <label className="field wide">Sign-in provider’s address (issuer URL)<input name="issuer" type="url" inputMode="url" placeholder="https://accounts.google.com" autoComplete="off" spellCheck={false} required /></label>
            <label className="field wide">Account identifier at that provider (subject)<input name="subject" autoComplete="off" spellCheck={false} required /></label>
            <p className="note">Use the provider's exact subject identifier, not the person's name or email address.</p>
          </div>
        </RecordedForm>
        {own ? null : <PersonAccount id={id} />}
      </> : null}
    </>} />
  </section>;
}

export function PersonSessions({ id }: { id: string }) {
  const me = useLoad(api.me, 'person-sessions-caller:' + id);
  return <section className="card"><h2>Signed-in sessions</h2><p>Browser sign-ins for this person. Ending a session signs that browser out; it does not stop agent processes.</p>
    <Gate load={me} title="Sessions" ok={(caller) => <SessionList key={id} person={caller.person.id === id ? '' : id} />} />
  </section>;
}

function MoveLogin({ id, login, people, moved }: {
  id: string; login: { issuer: string; subject: string }; people: PersonSummary[]; moved: (words: string) => void;
}) {
  const [open, setOpen] = useState(false);
  const [busy, setBusy] = useState(false);
  const [failure, setFailure] = useState('');
  const [answer, setAnswer] = useState('');
  return <div>
    <Act symbol="open" name="This sign-in belongs to someone else" word="Not theirs" onClick={() => setOpen(true)} disabled={busy} />
    {open ? <form aria-label="Move sign-in" onSubmit={(event) => {
      event.preventDefault();
      if (busy) return;
      const data = new FormData(event.currentTarget);
      const person = String(data.get('person') ?? '');
      const displayName = String(data.get('display_name') ?? '').trim();
      if (person && displayName || !person && !displayName) { setFailure('Choose an existing person or give a new name.'); return; }
      setBusy(true); setFailure('');
      request<{ source_grants: string[]; target_grants: string[] }>('/people/' + encodeURIComponent(id) + '/logins/move', {
        operation: operationId(), issuer: login.issuer, subject: login.subject,
        target: person ? { person } : { display_name: displayName },
      }).then((answer) => {
        const words = `Sign-in moved. This person still holds: ${answer.source_grants.join(', ') || 'no grants'}. The target holds: ${answer.target_grants.join(', ') || 'no grants'}.`;
        setAnswer(words); setOpen(false); moved(words);
      }, (failure: unknown) => setFailure(reasonOf(failure))).finally(() => setBusy(false));
    }}>
      <label>Existing person<select name="person" defaultValue=""><option value="">Choose a person</option>{people.filter((person) => person.id !== id).map((person) => <option key={person.id} value={person.id}>{person.display_name}</option>)}</select></label>
      <label>Or a new person’s name<input name="display_name" autoComplete="off" /></label>
      <p>The target keeps their existing sign-ins. Grants stay with their people. Sessions begun with this sign-in will end.</p>
      <Act symbol="approve" name="Move sign-in" word="Move" tone="primary" type="submit" disabled={busy} />
      <Act symbol="close" name="Cancel" word="Cancel" disabled={busy} onClick={() => setOpen(false)} />
    </form> : null}
    {failure ? <p role="alert">{failure}</p> : null}
    {answer ? <p role="status">{answer}</p> : null}
  </div>;
}
