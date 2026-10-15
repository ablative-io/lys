/**
 * How a grant form says what the service answered: a refusal in plain words, with the people and grants it names
 * by name, and its code only in the Error details fold; an unconfirmed change as "Not confirmed yet" with what to do.
 */
import type { ReactNode } from 'react';
import type { Refused } from '../../api';
import { problemWords } from '../people/Words';
import { named } from './model';
import type { GrantWorld } from './model';

export function RefusedWords({ w, id, refused }: { w: GrantWorld; id: string; refused: Refused }) {
  const { refusal, reason } = refused.refusal;
  return <div className="why-not" id={id} role="alert" data-refusal={refusal}>
    {/* A refusal this browser made itself (nothing reached Lys, or Lys's answer did not confirm the change) is said as it is, never as Lys refusing. */}
    <b>{named(w, refused.status === 0 || refused.status === 200 ? reason : problemWords(refused))}</b>
    {/* The fold holds the refusal exactly as Lys answered it, word for word (API refusal parity). */}
    <details><summary>Error details</summary><p className="refusal-name" style={{ overflowWrap: 'anywhere' }}>{reason.startsWith(refusal + ':') ? reason : refusal + ': ' + reason}</p></details>
  </div>;
}

export function PendingWords({ id, children }: { id: string; children: ReactNode }) {
  return <div className="why-not" id={id} role="status" data-pending="" style={{ color: 'var(--warn)' }}>
    <b>Not confirmed yet</b>
    {children}
  </div>;
}
