/** A person's credentials are sign-in bindings; their sessions are browser sign-ins, never agent runtimes. */
import { api, useLoad } from '../../api';
import { SessionList } from '../sessions/Sessions';
import { DirectoryGate as Gate } from '../people/Words';

export function PersonCredentials({ id }: { id: string }) {
  const load = useLoad(async () => {
    const me = await api.me();
    const logins = me.person.id === id ? me.sign_in_identities.map((login) => ({ issuer: login.provider, subject: login.subject })) : (await api.identity(id)).logins;
    return { logins, directory: (await api.people()).scope === 'directory' };
  }, 'person-credentials:' + id);
  return <section className="card"><h2>Sign-in accounts</h2><p>These accounts sign this person into Lys. Passwords and secret values are never shown here.</p>
    <Gate load={load} title="Sign-in accounts" ok={({ logins, directory }) => <>
      {logins.length ? <table><thead><tr><th>Sign-in provider</th><th>Account identifier</th></tr></thead><tbody>{logins.map((login) => <tr key={JSON.stringify([login.issuer, login.subject])}><td>{login.issuer}</td><td>{login.subject}</td></tr>)}</tbody></table> : <p>No sign-in account is bound to this person.</p>}
      {directory ? <p><a className="btn" href={'#/directory/manage?action=login&identity=' + encodeURIComponent(id)}>Connect a sign-in account</a>{' '}
        <a className="btn" href={'#/account/' + encodeURIComponent(id)}>Lys account: email, password, sign-in</a></p> : null}
    </>} />
  </section>;
}

export function PersonSessions({ id }: { id: string }) {
  const me = useLoad(api.me, 'person-sessions-caller:' + id);
  return <section className="card"><h2>Signed-in sessions</h2><p>Browser sign-ins for this person. Ending a session signs that browser out; it does not stop agent processes.</p>
    <Gate load={me} title="Sessions" ok={(caller) => <SessionList key={id} person={caller.person.id === id ? '' : id} />} />
  </section>;
}
