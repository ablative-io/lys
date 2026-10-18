/**
 * Who can use an approved app: everyone holding a standing grant on one of the app's kinds (an app kind is `app.kind`),
 * with the next step, giving people access, as a link. Approval lets people sign in to the app; until each holds a grant
 * on it, they can do nothing there, so this is said on the app's card rather than left to be found on Access.
 * Only the root authority sees every grant (the service answers it the whole directory), so "nobody" is said only to
 * them; anyone else is told the count covers the grants they can see.
 */
import { api, useLoad } from '../../api';
import type { Grant } from '../../generated/grants';
import { isRootAuthority } from '../grants/IssueRoot';
import { ErrorWords, IdentityName } from '../people/Words';

/** The identities holding a standing grant on one of `app`'s kinds, each once, in the order the grants list them. */
export function holdersOf(app: string, grants: Grant[]): string[] {
  const held = grants.filter((grant) => grant.standing.stands && grant.resource.kind.startsWith(app + '.')).map((grant) => grant.holder);
  return [...new Set(held)];
}

export function WhoCanUse({ app, name }: { app: string; name: string }) {
  const load = useLoad(() => Promise.all([api.grants(), api.people()]), 'who-can-use:' + app);
  const give = (primary: boolean) => <a className={'btn' + (primary ? ' primary' : '')} href={'#/access/issue?app=' + encodeURIComponent(app)}>Give people access</a>;
  if (load.status === 'loading') return <section className="app-users" aria-label={'Who can use ' + app}><b>Who can use {name}</b><p className="dim">Reading who holds {name} access…</p></section>;
  if (load.status === 'refused') return <section className="app-users" aria-label={'Who can use ' + app}>
    <b>Who can use {name}</b>
    <p className="note">Lys could not read who holds {name} access, so this card cannot say who can use it. Giving access does not depend on this list.</p>
    <ErrorWords problem={load.refused} />
    <div className="app-actions">{give(true)}</div>
  </section>;
  const [{ grants }, people] = load.data;
  const holders = holdersOf(app, grants);
  const whole = isRootAuthority(people);
  return <section className="app-users" aria-label={'Who can use ' + app}>
    <b>Who can use {name}</b>
    {holders.length ? <>
      <p className="note">{whole ? '' : 'Of the access you can see: '}{holders.length === 1 ? 'one person or agent holds' : holders.length + ' people and agents hold'} {name} access. Anyone else can sign in to {name} but can do nothing there until they are given access.</p>
      <ul className="app-list">{holders.map((id) => <li key={id}><IdentityName id={id} people={people} /></li>)}</ul>
    </> : <p className="note">{whole ? 'Nobody yet.' : 'Nobody, of the access you can see.'} People can sign in to {name}, but they can do nothing there until they are given access.</p>}
    <div className="app-actions">{give(!holders.length)}</div>
  </section>;
}
