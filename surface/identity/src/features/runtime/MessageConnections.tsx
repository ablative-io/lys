/** Message graph controls expose incomplete pagination and unresolved identity bindings, with no automatic retries. */
import { useState } from 'react';
import { problemWords } from '../people/Words';
import { readMessagePage } from './message-connections';
import type { MessageRead } from './message-connections';
import { Act } from '../../shell/Act';

export function MessageConnections({ value, change }: { value: MessageRead; change: (value: MessageRead) => void }) {
  const [busy, setBusy] = useState(false);
  const [failure, setFailure] = useState('');
  const more = async () => {
    setBusy(true); setFailure('');
    try { change(await readMessagePage(value)); }
    catch (error) { setFailure('More message connections could not be read. ' + problemWords(error)); }
    finally { setBusy(false); }
  };
  return <section aria-label="Message connections">
    <p className="note">{value.messages.length.toLocaleString('en-AU')} {value.messages.length === 1 ? 'message' : 'messages'} read from the message service. {value.pending.length ? 'More pages remain.' : 'All pages in this read have been read.'} These identify recipients, not whether a person read the message or an agent consumed it.</p>
    {value.unmapped.length ? <p className="why-not" role="status">Some people in the message service are not linked to anyone in Lys, so their messages are not drawn: {value.unmapped.join(', ')}.</p> : null}
    {value.pending.length ? <Act symbol="add" name={busy ? 'Reading message connections…' : 'Load more message connections'} word={busy ? 'Reading…' : 'More'} disabled={busy} onClick={() => void more()} /> : null}
    {failure ? <p className="why-not" role="alert">{failure}</p> : null}
  </section>;
}
