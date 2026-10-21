/**
 * A stranger founds an organisation (DIRECTORY-111 R3), outside the
 * signed-in shell: the organisation's name, their own name and their
 * email, posted once to Lys with a fresh operation id. The answer is the
 * same to everyone and names no identity: Lys wrote to the address, and
 * the letter's link proves it. A refusal is shown in words.
 *
 * On an install with no mail settings (DIRECTORY-112) the answer hands the
 * link instead, and the page shows it; an address that already signs in is
 * told so here, since no letter can.
 */
import { useRef, useState } from 'react';
import type { FormEvent } from 'react';
import { Refused, operationId, request } from '../../api';
import type { OrganisationFounded } from '../../generated';
import '../people/recorded-form.css';
import '../sign-in/sign-in.css';
import { Act } from '../../shell/Act';
import { LysMark } from '../../shell/LysMark';

/** A founding refusal in the words a person reads, without its name. */
export function foundingReasonOf(failure: unknown): string {
  if (failure instanceof Refused) {
    if (failure.status === 0) return 'Lys could not be reached. Check your connection and try again.';
    return failure.refusal.reason.replace(/^[A-Za-z_]+: /, '');
  }
  return String(failure);
}

export function Found() {
  const heading = useRef<HTMLHeadingElement>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState('');
  const [written, setWritten] = useState<{ email: string; organisation: string; letter: OrganisationFounded['letter'] } | null>(null);
  // One operation per page: asking again with the same words sends nothing twice.
  const operation = useRef(operationId());

  async function submit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    const form = new FormData(event.currentTarget);
    const organisation = String(form.get('organisation') ?? '').trim();
    const display_name = String(form.get('display_name') ?? '').trim();
    const email = String(form.get('email') ?? '').trim();
    if (!organisation || !display_name || !email) {
      setError("Enter the organisation's name, your name and your email.");
      return;
    }
    setBusy(true);
    setError('');
    try {
      const answer = await request<OrganisationFounded>('/organisations/found', { operation: operation.current, organisation, display_name, email });
      setWritten({ email, organisation, letter: answer.letter });
    } catch (failure) {
      setError(foundingReasonOf(failure));
    } finally {
      setBusy(false);
    }
  }

  if (written?.letter.handed !== undefined) {
    return <div className="page sign-in found">
      <LysMark />
      <h1 ref={heading} tabIndex={-1}>Your link</h1>
      <p role="status" className="sub">This Lys cannot send email, so your link is here instead of in a letter. Open it to finish founding {written.organisation}; it works once.</p>
      <p><a className="handed-link" href={written.letter.handed}>{written.letter.handed}</a></p>
    </div>;
  }
  if (written && written.letter.letter === 'already_yours' && !written.letter.sent) {
    return <div className="page sign-in found">
      <LysMark />
      <h1 ref={heading} tabIndex={-1}>Found an organisation</h1>
      <p role="status" className="sub">That address already signs in to Lys. <a href="#/sign-in">Sign in instead</a>.</p>
    </div>;
  }
  if (written) {
    return <div className="page sign-in found">
      <LysMark />
      <h1 ref={heading} tabIndex={-1}>Check your email</h1>
      <p role="status" className="sub">Lys wrote to {written.email}. Open the letter and follow its link to prove the address and finish founding {written.organisation}.</p>
    </div>;
  }
  return <div className="page sign-in found">
    <LysMark />
    <h1 ref={heading} tabIndex={-1}>Found an organisation</h1>
    <p className="sub">Lys writes to your email with a link; opening it proves the address and finishes the founding. You become the organisation's administrator.</p>
    <form className="recorded-form sign-in-form" aria-label="Found an organisation" onSubmit={submit} aria-busy={busy} noValidate>
      <div className="field">
        <label htmlFor="found-organisation">Organisation</label>
        <input id="found-organisation" name="organisation" type="text" autoComplete="organization" required disabled={busy} />
      </div>
      <div className="field">
        <label htmlFor="found-display-name">Your name</label>
        <input id="found-display-name" name="display_name" type="text" autoComplete="name" required disabled={busy} />
      </div>
      <div className="field">
        <label htmlFor="found-email">Your email</label>
        <input id="found-email" name="email" type="email" autoComplete="email" required disabled={busy} />
      </div>
      {error ? <p role="alert" className="why-not">{error}</p> : null}
      <Act symbol="send" name={busy ? 'Founding…' : 'Found'} word={busy ? 'Founding…' : 'Found'} tone="primary" type="submit" disabled={busy} />
    </form>
    <p className="note">Already have an account? <a href="#/sign-in">Sign in</a>.</p>
  </div>;
}
