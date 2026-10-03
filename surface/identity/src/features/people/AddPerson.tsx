import { useRef, useState } from 'react';
import { useNavigate } from 'react-router';
import { api, operationId, Refused, request, useLoad } from '../../api';
import { sendKept } from '../../kept';
import { DirectoryGate, ErrorWords } from './Words';
import { confirmReceipt } from './recorded-receipt';

interface PendingPerson { name: string; operation: string }
function object(value: unknown): value is Record<string, unknown> { return value !== null && typeof value === 'object' && !Array.isArray(value); }
function restored(key: string): PendingPerson | null {
  const raw = sessionStorage.getItem(key);
  if (raw === null) return null;
  const value: unknown = JSON.parse(raw);
  if (!object(value) || typeof value.name !== 'string' || !value.name.trim() || typeof value.operation !== 'string' || !/^op-[0-9a-f]{32}$/.test(value.operation)) {
    throw new Error('The saved request cannot be read. Check whether the person was added before submitting another request.');
  }
  return { name: value.name, operation: value.operation };
}

function PersonForm({ person }: { person: string }) {
  const navigate = useNavigate();
  const key = 'lys.add-person.' + person;
  const [saved] = useState(() => {
    try {
      if (sessionStorage.getItem('lys.pending.register-person') !== null) throw new Error('An earlier request still needs its outcome checked before adding another person. Ask the administrator to resolve that saved request.');
      return { pending: restored(key), error: null };
    } catch (error) { return { pending: null, error }; }
  });
  const [pending, setPending] = useState(saved.pending);
  const [name, setName] = useState(saved.pending?.name ?? '');
  const [failure, setFailure] = useState<unknown>(saved.error);
  const [busy, setBusy] = useState(false);
  const sending = useRef(false);
  const submit = async () => {
    if (sending.current || saved.error || !name.trim()) return;
    sending.current = true; setBusy(true); setFailure(null);
    try {
      const asked = pending ?? { name: name.trim(), operation: operationId() };
      const added = await sendKept(key, asked, async () => {
        setPending(asked);
        const answer = await request<unknown>('/people', { operation: asked.operation, display_name: asked.name });
        confirmReceipt(answer, asked.operation, '/people');
        if (!object(answer) || typeof answer.person !== 'string' || !/^person-[0-9a-f]{32}$/.test(answer.person)
          || !object(answer.receipt) || answer.receipt.identity !== answer.person || answer.receipt.change_kind !== 1) {
          throw new Refused(200, { refusal: 'UnconfirmedReceipt', reason: 'The answer did not confirm the person added. The original request is saved.' });
        }
        return answer.person;
      }, false);
      navigate('/file/' + encodeURIComponent(added), { replace: true });
    } catch (error) { setFailure(error); }
    finally { sending.current = false; setBusy(false); }
  };
  return <form aria-label="Add a person" onSubmit={(event) => { event.preventDefault(); void submit(); }}>
    <label className="field">Name<input name="display_name" autoFocus autoComplete="name" value={name} disabled={busy || pending !== null || Boolean(saved.error)} onChange={(event) => setName(event.target.value)} /></label>
    {pending ? <p role="status">This request has no confirmed answer yet. Its original details are saved; trying again checks the same request.</p> : null}
    {failure ? <ErrorWords problem={failure} /> : null}
    <p><button className="btn primary" type="submit" disabled={busy || !name.trim() || Boolean(saved.error)}>{busy ? 'Adding…' : pending ? 'Continue adding this person' : 'Add person'}</button></p>
  </form>;
}

export function AddPerson() {
  const load = useLoad(async () => {
    const [me, people] = await Promise.all([api.me(), api.people()]);
    return { me, people };
  }, 'add-person');
  return <div className="page"><a href="#/people">People and agents</a><h1>Add a person</h1>
    <DirectoryGate load={load} title="Add a person" ok={(data) => data.people.scope === 'directory'
      ? <PersonForm person={data.me.person.id} /> : <p>Only the Lys administrator can add people.</p>} />
  </div>;
}
