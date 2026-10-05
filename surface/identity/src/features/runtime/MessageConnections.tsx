/** Message graph controls expose incomplete pagination and unresolved identity bindings, with no automatic retries. */
import { useState } from 'react';
import { Refused } from '../../api';
import { readMessagePage } from './message-connections';
import type { MessageRead } from './message-connections';
import { Act } from '../../shell/Act';

export function MessageConnections({ value, change }: { value: MessageRead; change: (value: MessageRead) => void }) {
  const [busy, setBusy] = useState(false);
  const [failure, setFailure] = useState('');
  const more = async () => {
    setBusy(true); setFailure('');
    try { change(await readMessagePage(value)); }
    catch (error) { setFailure(error instanceof Refused ? error.refusal.refusal + ': ' + error.refusal.reason : String(error)); }
    finally { setBusy(false); }
  };
  return <section aria-label="Message connections">
    <p className="note">{value.messages.length} addressed messages read from the message service. {value.pending.length ? 'More pages remain.' : 'All pages in this read have been read.'} These identify recipients, not whether a person read the message or an agent consumed it.</p>
    {value.unmapped.length ? <p className="why-not" role="status">Identity bindings missing for message service participants: {value.unmapped.join(', ')}. Their connections are not drawn.</p> : null}
    {value.pending.length ? <Act symbol="add" name={busy ? 'Reading message connections…' : 'Load more message connections'} word={busy ? 'Reading…' : 'More'} disabled={busy} onClick={() => void more()} /> : null}
    {failure ? <p className="why-not" role="alert">{failure}</p> : null}
  </section>;
}
