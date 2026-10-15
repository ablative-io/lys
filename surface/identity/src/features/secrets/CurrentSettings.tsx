/** Read current broker settings without treating a read as a receipt for an uncertain write. */
import { useLoad } from '../../api';
import { Gate } from '../signin/Gate';
import { ScopeName } from './SecretsDetail';
import { secretsApi } from './secretsApi';

export function CurrentSettings({ secret, revision }: { secret: string; revision: number }) {
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
      <p>Visible to: {settings.scope === null ? 'not set yet' : <ScopeName scope={settings.scope} />}.</p>
      <p>May be handed to: {settings.recipients === 'people_only' ? 'people only' : 'anyone who is permitted'}.</p>
    </>} />
    <p className="note">This is what Lys secrets holds now. A change whose outcome is not known stays held below until it is confirmed.</p>
  </section>;
}
