/**
 * The secrets table's own last row, which adds one: its name, its kind, where it is used (the address its calls go
 * to, the header it travels in and the text before it there), and its value. The value is sent once, to the broker,
 * which seals it; it is never kept in this tab, never shown again, and never sent twice. What else was entered is kept
 * until the addition's outcome is known, so an addition with no answer is asked about, not sent again.
 */
import { useRef, useState } from 'react';
import type { FormEvent } from 'react';
import { operationId, Refused } from '../../api';
import { answeredNo, keepRecord, releaseRecord } from '../../kept';
import { secretsApi } from './secretsApi';
import type { AddedClass, SecretAdded, SecretAsked } from './secretsApi';

/** What a pending addition keeps: everything but the value. */
export interface PendingAddition { version: 1; operation: string; asked: SecretAsked }

/** Where this tab keeps a person's pending addition. */
export const additionKey = (person: string): string => 'lys.pending.secrets.add.' + person;

const isText = (value: unknown): value is string => typeof value === 'string';
const plain = (text: string): boolean => text.trim() !== '' && !/[\u0000-\u001f\u007f]/.test(text);

/** The pending addition kept under `key`, read in the one shape it is written in; any other shape is named, never guessed at. */
export function savedAddition(key: string): PendingAddition | null {
  const raw = sessionStorage.getItem(key);
  if (raw === null) return null;
  let value: unknown;
  try { value = JSON.parse(raw); } catch { value = null; }
  const asked = value && typeof value === 'object' && 'asked' in value ? value.asked : null;
  if (!value || typeof value !== 'object' || !('version' in value) || value.version !== 1 || !('operation' in value) || !isText(value.operation)
    || !asked || typeof asked !== 'object' || !('name' in asked) || !isText(asked.name) || !('class' in asked) || (asked.class !== 'credential' && asked.class !== 'key')
    || !('upstream' in asked) || !isText(asked.upstream) || !('header' in asked) || !isText(asked.header) || !('prefix' in asked) || !isText(asked.prefix)) {
    throw new Refused(0, { refusal: 'PendingSecretUnreadable', reason: 'The kept secret addition cannot be read. Remove ' + key + ' from this tab before adding another secret.' });
  }
  return { version: 1, operation: value.operation, asked: { name: asked.name, class: asked.class, upstream: asked.upstream, header: asked.header, prefix: asked.prefix } };
}

/** An answer that says exactly what was asked, or an outcome that is not known. */
export function confirmAdded(answer: unknown, pending: PendingAddition): SecretAdded {
  if (answer && typeof answer === 'object' && 'secret' in answer && answer.secret === pending.asked.name && 'operation' in answer
    && answer.operation === pending.operation && 'class' in answer && answer.class === pending.asked.class && 'owner' in answer && isText(answer.owner)
    && 'sequence' in answer && typeof answer.sequence === 'number' && 'repeated' in answer && typeof answer.repeated === 'boolean') {
    return { secret: pending.asked.name, class: pending.asked.class, owner: answer.owner, sequence: answer.sequence, operation: pending.operation, repeated: answer.repeated };
  }
  throw new Refused(200, { refusal: 'UnconfirmedAnswer', reason: 'The service answered, but not with ' + pending.asked.name + ' added.' });
}

/** A refusal of an addition in the row's own words. */
export function additionWords(error: unknown, name: string): string {
  if (!(error instanceof Refused)) return String(error);
  switch (error.refusal.refusal) {
    case 'SecretRetired': return 'The name ' + name + ' belonged to a secret that was retired, and a retired name is never used again. Choose another name.';
    case 'SecretExists': return 'A secret named ' + name + ' exists already. Choose another name, or change that secret’s value from its row.';
    case 'ValueEmpty': return 'Type the value to add.';
    case 'RouteInvalid': return 'Where it is used: ' + error.refusal.reason;
    default: return error.refusal.refusal + ': ' + error.refusal.reason;
  }
}

/** What the broker holds now of a pending addition: added by it, absent, or a secret of that name from another change. */
export async function checkAddition(pending: PendingAddition): Promise<'added' | 'absent' | 'other'> {
  try {
    const settings = await secretsApi.settings(pending.asked.name);
    return settings.last_operation === pending.operation ? 'added' : 'other';
  } catch (error) {
    if (error instanceof Refused && error.refusal.refusal === 'SecretUnknown') return 'absent';
    throw error;
  }
}

const EMPTY: SecretAsked = { name: '', class: 'credential', upstream: '', header: 'authorization', prefix: 'Bearer ' };

export function AddSecretRow({ person, changed }: { person: string; changed: (message: string) => void }) {
  const key = additionKey(person);
  const [restored] = useState(() => {
    try { return { pending: savedAddition(key), error: '' }; }
    catch (error) { return { pending: null, error: error instanceof Refused ? error.refusal.refusal + ': ' + error.refusal.reason : String(error) }; }
  });
  const [pending, setPending] = useState<PendingAddition | null>(restored.pending);
  const [asked, setAsked] = useState<SecretAsked>(restored.pending?.asked ?? EMPTY);
  const [value, setValue] = useState('');
  const [failure, setFailure] = useState(restored.error);
  const [busy, setBusy] = useState(false);
  const working = useRef(false);
  const release = () => { releaseRecord(key); setPending(null); };
  const added = (answer: SecretAdded) => {
    release(); setAsked(EMPTY);
    changed(answer.secret + ' was added. Its value is sealed and will not be shown again.');
  };
  const submit = async (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    if (working.current || pending || restored.error) return;
    const typed = { ...asked, name: asked.name.trim(), upstream: asked.upstream.trim(), header: asked.header.trim() };
    if (!plain(typed.name)) { setFailure('Give the secret a name, without control characters.'); return; }
    if (!plain(typed.upstream)) { setFailure('Say where it is used: the address its calls go to.'); return; }
    if (!plain(typed.header)) { setFailure('Say which header it travels in.'); return; }
    if (!value) { setFailure('Type the value to add.'); return; }
    const sending: PendingAddition = { version: 1, operation: operationId(), asked: typed };
    try { keepRecord(key, sending); } catch (error) { setFailure('The addition could not be kept in this tab, so it was not sent: ' + String(error)); return; }
    working.current = true; setBusy(true); setFailure(''); setPending(sending);
    const typedValue = value;
    // The value leaves the form as it is sent: it is never kept, so it is never sent again.
    setValue('');
    try {
      added(confirmAdded(await secretsApi.add(typed, typedValue, sending.operation), sending));
    } catch (error) {
      if (answeredNo(error) && error instanceof Refused) release();
      setFailure(additionWords(error, typed.name));
    } finally { working.current = false; setBusy(false); }
  };
  const check = async () => {
    if (!pending || working.current) return;
    working.current = true; setBusy(true); setFailure('');
    try {
      const found = await checkAddition(pending);
      if (found === 'added') {
        release(); setAsked(EMPTY);
        changed(pending.asked.name + ' was added. Its value is sealed and will not be shown again.');
      } else if (found === 'absent') {
        release(); setAsked(pending.asked);
        setFailure(pending.asked.name + ' was not added. Type its value again to add it.');
      } else {
        release();
        setFailure('A secret named ' + pending.asked.name + ' exists, but not from this addition. Choose another name.');
      }
    } catch (error) {
      setFailure('Lys could not tell whether ' + pending.asked.name + ' was added: ' + additionWords(error, pending.asked.name));
    } finally { working.current = false; setBusy(false); }
  };
  const blocked = busy || pending !== null || Boolean(restored.error);
  const id = 'add-secret-' + person.replace(/[^A-Za-z0-9_-]/g, '_');
  const set = (part: Partial<SecretAsked>) => setAsked({ ...asked, ...part });
  return <tr className="listing-add" data-add="secret">
    <td><form id={id} aria-label="Add a secret" onSubmit={(event) => { void submit(event); }}>
      <input name="name" aria-label="Name of the secret to add" placeholder="Name" value={asked.name} disabled={blocked} onChange={(event) => set({ name: event.target.value })} />
    </form></td>
    <td><select form={id} name="class" aria-label="Kind of secret" value={asked.class} disabled={blocked} onChange={(event) => set({ class: event.target.value === 'key' ? 'key' : 'credential' as AddedClass })}>
      <option value="credential">A credential</option><option value="key">A key</option>
    </select></td>
    <td>
      <input form={id} name="upstream" aria-label="Where it is used" placeholder="https://…" value={asked.upstream} disabled={blocked} onChange={(event) => set({ upstream: event.target.value })} />
      <input form={id} name="header" aria-label="Header it travels in" value={asked.header} disabled={blocked} onChange={(event) => set({ header: event.target.value })} />
      <input form={id} name="prefix" aria-label="Text before it in that header" value={asked.prefix} disabled={blocked} onChange={(event) => set({ prefix: event.target.value })} />
    </td>
    <td>
      <input form={id} name="value" type="password" autoComplete="new-password" aria-label="Value" placeholder="Value, shown never again" value={value} disabled={blocked} onChange={(event) => setValue(event.target.value)} />
      {pending ? <div role="status"><p>Adding {pending.asked.name} is not confirmed. What you entered is kept, except the value, which is never kept.</p>
        <button className="btn" type="button" disabled={busy} onClick={() => { void check(); }}>Check whether it was added</button></div> : null}
      {failure ? <p className="why-not" role="alert">{failure}</p> : null}
    </td>
    <td><button form={id} className="btn primary" type="submit" disabled={blocked}>Add this secret</button></td>
  </tr>;
}
