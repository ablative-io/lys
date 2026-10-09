import { useRef, useState } from 'react';
import { api, operationId, Refused, request, useLoad } from '../../api';
import type { GrantModel } from '../../generated/grants';
import { sendKept } from '../../kept';
import { DirectoryGate, ErrorWords } from './Words';
import './add-agent.css';
import { Act } from '../../shell/Act';

/** Adding a person is one act, POST /people/admit: register them, make their sign-in account at the issuer by
 * email, bind it, activate them and issue their first grant. The answer is one receipt naming each step and the log
 * leaf it was recorded at; asking again with the saved request answers the same receipt and makes nothing twice. */
interface PendingPerson { name: string; email: string; kind: string; id: string; relation: string; operation: string }
interface Logged { step: string; log: string; index: number }
interface Admitted { person: string; logged: Logged[] }
const STEPS = ['register', 'account', 'bind', 'activate', 'grant'];
const WORDS: Record<string, string> = { register: 'Registered', bind: 'Sign-in bound', activate: 'Activated', grant: 'First grant issued' };

function object(value: unknown): value is Record<string, unknown> { return value !== null && typeof value === 'object' && !Array.isArray(value); }
function filled(value: Record<string, unknown>, key: string): string | null {
  const field = value[key];
  return typeof field === 'string' && field.trim() ? field : null;
}
function restored(key: string): PendingPerson | null {
  const raw = sessionStorage.getItem(key);
  if (raw === null) return null;
  const value: unknown = JSON.parse(raw);
  const unreadable = new Error('The saved request cannot be read. Check whether the person was added before submitting another request.');
  if (!object(value)) throw unreadable;
  const [name, email, kind, id, relation, operation] = ['name', 'email', 'kind', 'id', 'relation', 'operation'].map((each) => filled(value, each));
  if (!name || !email || !kind || !id || !relation || !operation || !/^op-[0-9a-f]{32}$/.test(operation)) throw unreadable;
  return { name, email, kind, id, relation, operation };
}

/** The act's answer, confirmed for `operation`: every step completed, none failed, each written step's leaf named. */
function confirmed(answer: unknown, operation: string): Admitted {
  const unconfirmed = new Refused(200, { refusal: 'UnconfirmedReceipt', reason: 'The answer did not confirm this person was added. The original request is saved.' });
  if (!object(answer) || typeof answer.person !== 'string' || !/^person-[0-9a-f]{32}$/.test(answer.person) || !object(answer.receipt)) throw unconfirmed;
  const receipt = answer.receipt;
  const logged = receipt.logged;
  if (receipt.operation !== operation || receipt.person !== answer.person || (receipt.failed !== null && receipt.failed !== undefined)
    || !Array.isArray(receipt.completed) || receipt.completed.join(' ') !== STEPS.join(' ') || !Array.isArray(logged)
    || !logged.every((leaf: unknown) => object(leaf) && typeof leaf.step === 'string' && typeof leaf.log === 'string'
      && typeof leaf.index === 'number' && Number.isSafeInteger(leaf.index) && leaf.index >= 0)) throw unconfirmed;
  return { person: answer.person, logged: logged as Logged[] };
}

function Receipt({ admitted }: { admitted: Admitted }) {
  return <section aria-label="Receipt" className="note">
    <p>Added. One act, recorded as:</p>
    <ul>
      {admitted.logged.map((leaf) => <li key={leaf.log + leaf.index}>{WORDS[leaf.step] ?? leaf.step}: {leaf.log} log leaf {leaf.index}</li>)}
      <li>Sign-in account: the issuer account for their email, found or made there; the issuer keeps that record</li>
    </ul>
    <a href={'#/file/' + encodeURIComponent(admitted.person)}>Open their file</a>
  </section>;
}

function PersonForm({ person, model }: { person: string; model: GrantModel }) {
  const key = 'lys.add-person.' + person;
  const [saved] = useState(() => {
    try {
      if (sessionStorage.getItem('lys.pending.register-person') !== null) throw new Error('An earlier request still needs its outcome checked before adding another person. Ask the administrator to resolve that saved request.');
      return { pending: restored(key), error: null };
    } catch (error) { return { pending: null, error }; }
  });
  const [pending, setPending] = useState(saved.pending);
  const [asking, setAsking] = useState<Omit<PendingPerson, 'operation'>>(saved.pending ?? { name: '', email: '', kind: '', id: '', relation: '' });
  const [failure, setFailure] = useState<unknown>(saved.error);
  const [admitted, setAdmitted] = useState<Admitted | null>(null);
  const [busy, setBusy] = useState(false);
  const sending = useRef(false);
  const complete = Object.values(asking).every((value) => value.trim());
  const set = (field: keyof typeof asking) => (event: { target: { value: string } }) => setAsking({ ...asking, [field]: event.target.value });
  const locked = busy || pending !== null || Boolean(saved.error);
  const submit = async () => {
    if (sending.current || saved.error || !complete) return;
    sending.current = true; setBusy(true); setFailure(null);
    try {
      const asked: PendingPerson = pending ?? { ...asking, name: asking.name.trim(), email: asking.email.trim(), operation: operationId() };
      const answer = await sendKept(key, asked, async () => {
        setPending(asked);
        return confirmed(await request<unknown>('/people/admit', {
          operation: asked.operation, display_name: asked.name, email: asked.email,
          grant: { route: 'browser', resource: { kind: asked.kind, id: asked.id }, relation: asked.relation },
        }), asked.operation);
      }, false);
      setPending(null); setAdmitted(answer);
    } catch (error) { setFailure(error); }
    finally { sending.current = false; setBusy(false); }
  };
  if (admitted) return <Receipt admitted={admitted} />;
  return <form aria-label="Add a person" onSubmit={(event) => { event.preventDefault(); void submit(); }}>
    <label className="field">Name<input name="display_name" autoFocus autoComplete="name" value={asking.name} disabled={locked} onChange={set('name')} /></label>
    <label className="field">Email<input name="email" type="email" autoComplete="email" value={asking.email} disabled={locked} onChange={set('email')} /></label>
    <p className="note">Their first grant: what they may use from the start. More can be given from Access later.</p>
    <label className="field">Kind of thing<input name="kind" autoComplete="off" value={asking.kind} disabled={locked} onChange={set('kind')} /></label>
    <label className="field">Which one<input name="resource" autoComplete="off" value={asking.id} disabled={locked} onChange={set('id')} /></label>
    <label className="field">Relation<select name="relation" value={asking.relation} disabled={locked} onChange={set('relation')}>
      <option value="" disabled>Choose a relation</option>
      {Object.entries(model.relations).map(([relation, actions]) => <option key={relation} value={relation}>{relation} · {actions.join(', ')}</option>)}
    </select></label>
    <Act symbol="add" name={busy ? 'Adding…' : pending ? 'Continue adding this person' : 'Add person'} word={busy ? 'Adding…' : pending ? 'Continue' : 'Add'} tone="primary" type="submit" disabled={busy || !complete || Boolean(saved.error)} />
    {pending ? <p role="status">This request has no confirmed answer yet. Its original details are saved; trying again checks the same request.</p> : null}
    {failure ? <ErrorWords problem={failure} /> : null}
  </form>;
}

export function AddPerson() {
  const load = useLoad(async () => {
    const [me, people, model] = await Promise.all([api.me(), api.people(), api.model()]);
    return { me, people, model };
  }, 'add-person');
  return <div className="page"><a href="#/people">People and agents</a><h1>Add a person</h1>
    <DirectoryGate load={load} title="Add a person" ok={(data) => data.people.scope === 'directory'
      ? <PersonForm person={data.me.person.id} model={data.model} /> : <p>Only the Lys administrator can add people.</p>} />
  </div>;
}
