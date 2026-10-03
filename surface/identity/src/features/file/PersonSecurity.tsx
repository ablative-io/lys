/** A person's credentials are sign-in bindings; their sessions are browser sign-ins, never agent runtimes. */
import { api, useLoad } from '../../api';
import { SessionList } from '../sessions/Sessions';
import { DirectoryGate as Gate } from '../people/Words';
import { RecordedForm } from '../people/RecordedForm';
import { PersonAccount } from '../people/Account';

export function PersonCredentials({ id }: { id: string }) {
  const [revision, setRevision] = useState(0);
  const load = useLoad(async () => {
    const me = await api.me();
    const logins = me.person.id === id ? me.sign_in_identities.map((login) => ({ issuer: login.provider, subject: login.subject })) : (await api.identity(id)).logins;
    return { logins, directory: (await api.people()).scope === 'directory' };
  }, 'person-credentials:' + id);
  return <section className="card"><h2>Sign-in accounts</h2><p>These accounts sign this person into Lys. Passwords and secret values are never shown here.</p>
    <Gate load={load} title="Sign-in accounts" ok={({ logins, directory }) => <>
      {logins.length ? <table><thead><tr><th>Sign-in provider</th><th>Account identifier</th></tr></thead><tbody>{logins.map((login) => <tr key={JSON.stringify([login.issuer, login.subject])}><td>{login.issuer}</td><td>{login.subject}</td></tr>)}</tbody></table> : <p>No sign-in account is bound to this person.</p>}
      {directory ? <>
        <RecordedForm name="bind-login" title="Bind a sign-in identity" heading="Connect a sign-in account" description="Connect this person's directory record to their account with a sign-in provider." submitLabel="Connect sign-in" done={() => setRevision((value) => value + 1)}
          change={(form) => ({ path: '/people/' + encodeURIComponent(id) + '/logins', body: { issuer: String(form.get('issuer') ?? '').trim(), subject: String(form.get('subject') ?? '').trim() } })}>
          <div className="form-grid">
            <label className="field">Issuer URL<input name="issuer" required /></label>
            <label className="field">Account subject<input name="subject" required /></label>
            <p className="note">Use the provider's exact subject identifier, not the person's name or email address.</p>
          </div>
        </RecordedForm>
        <PersonAccount id={id} />
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
