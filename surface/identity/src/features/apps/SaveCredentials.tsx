/** Credentials remain in memory until the broker confirms their encrypted references. */
import { useRef, useState } from 'react';
import { Refused } from '../../api';
import { send } from './SchemaBuilder';
import type { ClientIssued } from './Apps';

export type StoredCredentials = { app: string; client_secret_ref: string; api_credential_ref: string };
export async function saveCredentials(app: string, client: ClientIssued): Promise<StoredCredentials> {
  const answer = await send<StoredCredentials>('POST', '/apps/' + encodeURIComponent(app) + '/credentials/save', {client_secret:client.client_secret});
  if (answer?.app !== app || typeof answer.client_secret_ref !== 'string' || !answer.client_secret_ref || typeof answer.api_credential_ref !== 'string' || !answer.api_credential_ref) throw new Error('Lys secrets answered, but did not confirm it saved these two secrets');
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
    catch(error){setError(error instanceof Refused ? error.refusal.reason : error instanceof Error ? error.message : String(error));}
    finally{working.current=false;setBusy(false);}
  };
  return <section className="app-secret" aria-label="Save app credentials"><p>Approving made two secrets for this app: the secret it signs people in with, and its key for calling Lys. They are held on this page only, and are never shown. Save them before you leave.</p><button className="btn primary" type="button" disabled={busy} onClick={()=>void save()}>Save credentials in Lys secrets</button>
    {error?<div role="alert"><p>The save was not confirmed, so both secrets are still held on this page. Nothing is tried again on its own. Keep this page open and choose Save again: it sends the same two secrets and never replaces one already saved.</p><p className="sec">Lys said: {error}</p></div>:null}</section>;
}
export function SavedCredentials({answer}:{answer:StoredCredentials}) {
  return <section role="status"><p>Credentials saved in Lys secrets, owned by you. Nobody can read them back. The app's key is only ever used by Lys secrets, which adds it to calls on behalf of someone you allow to use that secret.</p><details><summary>Their names in Lys secrets</summary><p>Sign-in secret: {answer.client_secret_ref}</p><p>Key for calling Lys: {answer.api_credential_ref}</p></details></section>;
}
