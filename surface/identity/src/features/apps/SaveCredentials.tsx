/** Credentials remain in memory until the broker confirms their encrypted references. */
import { useRef, useState } from 'react';
import { Refused } from '../../api';
import { send } from './SchemaBuilder';
import type { ClientIssued } from './Apps';

export type StoredCredentials = { app: string; client_secret_ref: string; api_credential_ref: string };
export async function saveCredentials(app: string, client: ClientIssued): Promise<StoredCredentials> {
  const answer = await send<StoredCredentials>('POST', '/apps/' + encodeURIComponent(app) + '/credentials/save', {client_secret:client.client_secret});
  if (answer?.app !== app || typeof answer.client_secret_ref !== 'string' || !answer.client_secret_ref || typeof answer.api_credential_ref !== 'string' || !answer.api_credential_ref) throw new Error('The broker did not confirm the saved credentials');
  return answer;
}
export function SaveCredentials({app, client, saved}:{app:string; client:ClientIssued; saved:(answer:StoredCredentials)=>void}) {
  const working=useRef(false);
  const [busy,setBusy]=useState(false);
  const [error,setError]=useState('');
  const save=async()=>{
    if(working.current)return;
    working.current=true;setBusy(true);setError('');
    try{saved(await saveCredentials(app,client));}
    catch(error){setError((error instanceof Refused ? error.refusal.refusal+': '+error.refusal.reason : String(error))+'. No automatic retry. Keep this page open; Save again checks the same values without replacing an existing secret.');}
    finally{working.current=false;setBusy(false);}
  };
  return <section className="app-secret" aria-label="Save app credentials"><p>Keep these credentials in Lys secrets without copying them.</p><button className="btn primary" type="button" disabled={busy} onClick={()=>void save()}>Save credentials in Lys secrets</button>{error?<p role="alert">{error}</p>:null}
    <details><summary>Show credentials once</summary><div aria-label="Client secret, shown once"><dl><dt>Client id</dt><dd>{client.client_id}</dd><dt>Client secret</dt><dd className="mono">{client.client_secret}</dd><dt>API credential</dt><dd className="mono">{client.credential}</dd></dl></div></details></section>;
}
export function SavedCredentials({answer}:{answer:StoredCredentials}) {
  return <section role="status"><p>Credentials saved in Lys secrets. You do not need to copy them.</p><details><summary>Stored references</summary><p>Client secret: <code>{answer.client_secret_ref}</code></p><p>API credential: <code>{answer.api_credential_ref}</code></p><p>Use through the secrets broker requires a holder-bound handle and an explicit use grant.</p></details></section>;
}
