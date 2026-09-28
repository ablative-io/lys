/** A directory change stays pending after an uncertain answer, including across reloads. */
import { useRef, useState } from 'react';
import type { FormEvent, ReactNode } from 'react';
import { operationId, Refused, request } from '../../api';
import { confirmReceipt } from './recorded-receipt';
import './recorded-form.css';

export interface Change {
  path: string;
  body: Record<string, unknown>;
}

export function RecordedForm({ name, title, heading, description, submitLabel, children, change, done }: {
  name: string;
  title: string;
  heading?: string;
  description?: string;
  submitLabel?: string;
  children: ReactNode;
  change: (data: FormData) => Change;
  done: () => void;
}) {
  const key = 'lys.pending.' + name;
  const [pending, setPending] = useState(() => {
    const saved = sessionStorage.getItem(key);
    if (!saved) return null;
    try {
      const record: unknown = JSON.parse(saved);
      if (record && typeof record === 'object' && 'operation' in record && typeof record.operation === 'string') return record.operation;
    } catch {
      // A damaged pending record cannot establish that resubmission is safe.
    }
    return 'Pending record could not be read; inspect ' + key;
  });
  const [answer, setAnswer] = useState('');
  const [failure, setFailure] = useState('');
  const busy = useRef(false);
  const submit = async (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    if (busy.current || pending) return;
    busy.current = true;
    setFailure('');
    setAnswer('');
    try {
      const asked = change(new FormData(event.currentTarget));
      const operation = operationId();
      // Persist before sending. Failure to retain the request refuses the send.
      sessionStorage.setItem(key, JSON.stringify({ ...asked, operation }));
      setPending(operation);
      try {
        const result = await request<unknown>(asked.path, { ...asked.body, operation });
        confirmReceipt(result, operation, asked.path);
        setAnswer(JSON.stringify(result, null, 2));
        sessionStorage.removeItem(key);
        setPending(null);
        done();
      } catch (error) {
        if (error instanceof Refused && error.status >= 400 && error.status < 500) {
          sessionStorage.removeItem(key);
          setPending(null);
        }
        throw error;
      }
    } catch (error) {
      setFailure(error instanceof Refused ? `${error.refusal.refusal}: ${error.refusal.reason}` : String(error));
    } finally {
      busy.current = false;
    }
  };
  return <form className="card recorded-form" onSubmit={submit} aria-label={title}>
    <h2>{heading ?? title}</h2>
    {description ? <p className="recorded-description">{description}</p> : null}
    <fieldset disabled={pending !== null} style={{ border: 0, padding: 0 }}>
      {children}
      <button className="btn primary" type="submit">{submitLabel ?? title}</button>
    </fieldset>
    {pending ? <p role="status">Awaiting a confirmed result. Do not submit this change again. Its operation is retained in this browser: <code>{pending}</code>.</p> : null}
    {failure ? <p className="why-not" role="alert">{failure}</p> : null}
    {answer ? <details open><summary>Recorded receipt</summary><pre style={{ whiteSpace: 'pre-wrap', overflowWrap: 'anywhere' }}>{answer}</pre></details> : null}
  </form>;
}

export function TextField({ name, label, optional = false }: { name: string; label: string; optional?: boolean }) {
  return <div className="field"><label>{label}<input name={name} required={!optional} /></label></div>;
}

export function field(data: FormData, name: string): string {
  const value = data.get(name);
  if (typeof value !== 'string') throw new Error(`Missing ${name}`);
  return value.trim();
}
