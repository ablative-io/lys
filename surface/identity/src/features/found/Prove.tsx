/**
 * The proving page (DIRECTORY-111 R3), outside the signed-in shell: the
 * letter's link lands here with the proof in the address. A read names
 * whose record it opens before either act. Nothing is posted on load.
 * On success the founder is signed in by the answer's cookie and lands on
 * their account page to set a password; a refusal is shown in words with
 * the way back to sign-in.
 *
 * At #/added/:proof (DIRECTORY-112) the same page signs in a person whose
 * admission link was handed to them on an install with no mail settings,
 * landing on their account page told so. An admission a product asked for
 * answers where to go on to (DIRECTORY-113 R3); it is carried to the
 * account page in the navigation's state, never the address, so no link
 * can put an address on Lys's page.
 */
import { useEffect, useRef, useState } from 'react';
import { useNavigate, useParams } from 'react-router';
import { Refused, request } from '../../api';
import type { OrganisationProved } from '../../generated';
import '../people/recorded-form.css';
import '../sign-in/sign-in.css';
import { Act } from '../../shell/Act';
import { LysMark } from '../../shell/LysMark';
import { foundingReasonOf } from './Found';

/** Where a proven founder lands: their account page, told how they are signed in. */
export const FOUNDED = '/settings/account?founded=1';

/** Where a person signed in by a handed admission link lands. */
export const ADDED = '/settings/account?added=1';

export function Prove({ admission = false }: { admission?: boolean }) {
  const { proof = '' } = useParams();
  const navigate = useNavigate();
  const heading = useRef<HTMLHeadingElement>(null);
  const [busy, setBusy] = useState(false);
  const [refusal, setRefusal] = useState<{ name: string; words: string } | null>(null);
  const [whom, setWhom] = useState<{ display_name: string; masked_email: string } | null>(null);
  const [declined, setDeclined] = useState(false);
  useEffect(() => {
    let live = true;
    setWhom(null); setDeclined(false); setRefusal(null);
    request<{ display_name: string; masked_email: string }>('/organisations/found/' + encodeURIComponent(proof) + '/whom').then(
      (answer) => { if (live) setWhom(answer); },
      (failure: unknown) => { if (live) setRefusal({ name: failure instanceof Refused ? failure.refusal.refusal : '', words: foundingReasonOf(failure) }); },
    );
    return () => { live = false; };
  }, [proof]);
  const act = whom ? `I am ${whom.display_name}, sign me in` : 'Reading whose link this is…';

  async function finish() {
    if (busy || !whom || declined) return;
    setBusy(true);
    setRefusal(null);
    try {
      const proved = await request<OrganisationProved>('/organisations/found/prove', { proof });
      navigate(admission ? ADDED : FOUNDED, proved.continue_to ? { state: { continueTo: proved.continue_to } } : undefined);
    } catch (failure) {
      setRefusal({ name: failure instanceof Refused ? failure.refusal.refusal : '', words: foundingReasonOf(failure) });
    } finally {
      setBusy(false);
    }
  }

  return <div className="page sign-in found">
    <LysMark />
    <h1 ref={heading} tabIndex={-1}>{admission ? 'Sign in to Lys' : 'Finish founding'}</h1>
    <p className="sub">{whom ? `This link is for ${whom.display_name} (${whom.masked_email})` : 'Reading whose link this is…'}</p>
    {refusal ? <p role="alert" className="why-not">{refusal.words}{refusal.name === 'proof_refused' || refusal.name === 'already_admitted' ? <> <a href="#/sign-in">Go to sign in</a>.</> : null}</p> : null}
    {declined ? <p role="status">Ask the person who sent this link to send the right one. This link has not been used.</p> : whom ? <>
      <Act symbol="enter" name={busy ? 'Finishing…' : act} word={busy ? 'Finishing…' : act} tone="primary" disabled={busy || !proof} onClick={finish} />
      <Act symbol="close" name="This isn't me" word="This isn't me" disabled={busy} onClick={() => setDeclined(true)} />
    </> : null}
  </div>;
}
