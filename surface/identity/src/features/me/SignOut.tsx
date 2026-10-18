/**
 * Sign out: end this browser's own session and open the sign-in page, which says so. One press and no
 * confirmation, because signing out loses nothing a person typed and signing in undoes it. Lys has no
 * sign-out route of its own; this browser's session is the one `/sessions` marks current, ended as the
 * sessions list ends it. A failure says what happened and that the person is still signed in.
 */
import { useEffect, useState } from 'react';
import { Refused, request } from '../../api';
import { Act } from '../../shell/Act';
import { forgetKept, rememberSignedIn } from '../signin/fileReturn';
import { failureWords } from '../signin/words';
import './sign-out.css';

interface Sessions { sessions: { id: string; current: boolean }[] }

/** Where a person goes once signed out: the sign-in page, told why it is showing. */
export const signedOutPage = { open: (): void => location.assign('/#/sign-in?signed-out') };

/**
 * The sign-out control; `who` is the signed-in person's name, drawn beside it where a page does not already show
 * it. `person` is their identifier, noted for this browser so a later sign-in by someone else fills in nothing
 * this person left open (signin/fileReturn.tsx).
 */
export function SignOut({ who, person }: { who?: string; person?: string }) {
  const [busy, setBusy] = useState(false);
  const [problem, setProblem] = useState('');
  useEffect(() => {
    if (person) rememberSignedIn(person);
  }, [person]);
  const leave = () => {
    try { forgetKept(); } catch { /* A tab that keeps no storage kept no link either. */ }
    signedOutPage.open();
  };
  async function signOut() {
    if (busy) return;
    setBusy(true);
    setProblem('');
    try {
      const { sessions } = await request<Sessions>('/sessions');
      const current = sessions.find((session) => session.current);
      if (!current) throw new Error('Lys did not list this browser’s session, so it could not end it.');
      const answer = await request<{ ended: string }>('/sessions/' + encodeURIComponent(current.id) + '/end', {});
      if (answer.ended !== current.id) throw new Error('Lys answered, but its answer named a different session.');
      leave();
    } catch (error) {
      // Already signed out (the session expired or was ended elsewhere): the result the person asked for.
      if (error instanceof Refused && error.status === 401) leave();
      else setProblem(failureWords(error, 'You are still signed in. Press Sign out again.'));
    } finally {
      setBusy(false);
    }
  }
  return <div className="sign-out">
    {who ? <span className="sign-out-who">{who}</span> : null}
    <Act symbol="back" name={busy ? 'Signing out…' : 'Sign out'} word={busy ? 'Signing out…' : 'Sign out'} disabled={busy} onClick={() => { void signOut(); }} />
    {problem ? <p role="alert" className="why-not">{problem}</p> : null}
  </div>;
}
