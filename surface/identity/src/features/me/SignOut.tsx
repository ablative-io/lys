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

/** The links kept for a sign-in belong to the person leaving; then the sign-in page. */
function leave(): void {
  try { forgetKept(); } catch { /* A tab that keeps no storage kept no link either. */ }
  signedOutPage.open();
}

/**
 * End this browser's session and open the sign-in page: the one sign-out act, pressed from the rail, You and
 * Search alike. Answers the words to show when it could not, or null once the person has left.
 */
export async function signOutNow(): Promise<string | null> {
  try {
    const { sessions } = await request<Sessions>('/sessions');
    const current = sessions.find((session) => session.current);
    if (!current) throw new Error('Lys did not list this browser’s session, so it could not end it.');
    const answer = await request<{ ended: string }>('/sessions/' + encodeURIComponent(current.id) + '/end', {});
    if (answer.ended !== current.id) throw new Error('Lys answered, but its answer named a different session.');
    leave();
    return null;
  } catch (error) {
    // Already signed out (the session expired or was ended elsewhere): the result the person asked for.
    if (error instanceof Refused && error.status === 401) {
      leave();
      return null;
    }
    return failureWords(error, 'You are still signed in. Press Sign out again.');
  }
}

/**
 * The sign-out control; `who` is the signed-in person's name, drawn beside it where a page does not already show
 * it. `person` is their identifier, noted for this browser so a later sign-in by someone else fills in nothing
 * this person left open (signin/fileReturn.tsx). In the rail (`place="rail"`) it is drawn as a rail entry, and a
 * failure goes to `told` (the shell's toast), since the rail has no room for a sentence.
 */
export function SignOut({ who, person, place = 'page', told }: { who?: string; person?: string; place?: 'page' | 'rail'; told?: (words: string) => void }) {
  const [busy, setBusy] = useState(false);
  const [problem, setProblem] = useState('');
  useEffect(() => {
    if (person) rememberSignedIn(person);
  }, [person]);
  async function signOut() {
    if (busy) return;
    setBusy(true);
    setProblem('');
    const words = await signOutNow();
    setBusy(false);
    if (words === null) return;
    if (told) told(words);
    else setProblem(words);
  }
  const word = busy ? 'Signing out…' : 'Sign out';
  if (place === 'rail') {
    return <div className="sign-out-rail">
      <Act className="quiet" symbol="leave" name={word} hint={who ? word + ' (' + who + ')' : word} word={word} disabled={busy} onClick={() => { void signOut(); }} />
      {who ? <span className="lbl sign-out-who">{who}</span> : null}
    </div>;
  }
  return <div className="sign-out">
    {who ? <span className="sign-out-who">{who}</span> : null}
    <Act symbol="leave" name={busy ? 'Signing out…' : 'Sign out'} word={busy ? 'Signing out…' : 'Sign out'} disabled={busy} onClick={() => { void signOut(); }} />
    {problem ? <p role="alert" className="why-not">{problem}</p> : null}
  </div>;
}
