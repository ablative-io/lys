/**
 * A saved request this browser cannot send again, said on the page that is locked by it, with the one act that
 * unlocks the page. A readable saved request is never cleared here: its own form sends it again, the same request,
 * and only its answer settles it (kept.ts). What reaches this is a record that cannot be read back, or one kept in a
 * shape an earlier Lys wrote; neither can be sent again as it was, so what remains is to check the outcome where it
 * would show and then release the record. The record lives in this browser tab alone, so nobody else, the
 * administrator included, can clear it.
 */
import { useState } from 'react';
import type { ReactNode } from 'react';
import { releaseRecord } from '../../kept';
import { Act } from '../../shell/Act';
import { ErrorWords } from './Words';

export function ClearSavedRequest({ what, check, keys, cleared }: {
  /** The request in a few words: "request to add a person". */
  what: string;
  /** Where its outcome would show, said as a sentence with its link. */
  check: ReactNode;
  /** The browser records released by the act. */
  keys: string[];
  cleared: () => void;
}) {
  const [failure, setFailure] = useState<unknown>(null);
  const clear = () => {
    setFailure(null);
    try {
      for (const key of keys) releaseRecord(key);
    } catch (error) {
      setFailure(new Error('SavedRequestUnreleased: this browser would not remove the saved request. ' + (error instanceof Error ? error.message : String(error))));
      return;
    }
    cleared();
  };
  return <section className="why-not" role="alert" aria-label="Saved request">
    <p>This browser kept an earlier {what} whose outcome is not known, and it cannot be sent again as it was kept. Until it is checked and cleared, this form stays locked.</p>
    <p>{check}</p>
    <p><Act symbol="retire" name="Check and clear the saved request" word="Clear saved request" tone="primary" onClick={clear} /></p>
    {failure ? <ErrorWords problem={failure} /> : null}
  </section>;
}
