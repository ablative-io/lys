/** Read current broker settings without treating a read as a receipt for an uncertain write. */
import { useState } from 'react';
import { useLoad } from '../../api';
import { Gate } from '../signin/Gate';
import { scopeWords } from './SecretsDetail';
import { secretsApi } from './secretsApi';

export function CurrentSettings({ secret }: { secret: string }) {
  const [revision, setRevision] = useState(0);
  const load = useLoad(async () => {
    const answer = await secretsApi.settings(secret);
    if (answer.secret !== secret || !(answer.scope === null || typeof answer.scope === 'string')
      || (answer.recipients !== 'anyone' && answer.recipients !== 'people_only')) {
      throw new Error('The settings answer does not describe ' + secret);
    }
    return answer;
  }, 'secret-settings:' + secret + ':' + revision);
  return <section className="card" aria-label="Current secret settings">
    <h2>Current settings</h2><Gate load={load} title="Secret settings" ok={(settings) => <>
      <p>Visible to: {settings.scope === null ? 'no scope recorded' : scopeWords(settings.scope)}.</p>
      <p>May be handed to: {settings.recipients === 'people_only' ? 'people only' : 'anyone who is permitted'}.</p>
    </>} />
    <button type="button" className="btn" disabled={load.status === 'loading'} onClick={() => setRevision((value) => value + 1)}>Refresh current settings</button>
    <p className="note">These are the broker's current values. A change with an unknown outcome remains held until its own result is confirmed.</p>
  </section>;
}
