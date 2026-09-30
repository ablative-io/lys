/**
 * Secrets screens beyond the listing: grants, the audit log, where a
 * handle's revocation stands, and an owner's changes to a secret's scope and
 * recipients. Every screen renders chosen metadata fields only; a secret's
 * value, a handle, a key, an upstream address or a header never reaches the
 * page, whatever else an answer carries.
 */
import { useRef, useState } from 'react';
import type { FormEvent } from 'react';
import { Refused, useLoad } from '../../api';
import { clock } from '../file/time';
import { Gate } from '../signin/Gate';
import type {
  Recipients,
  RecipientsChanged,
  RevocationAnswer,
  ScopeChanged,
  ScopeKind,
  SecretAuditKind,
  SecretAuditLog,
  SecretGrantListing,
} from './secretsApi';
import { pendingKey, unconfirmed } from './pendingChange';
import { useOwnerChange } from './useOwnerChange';
import type { PendingStore } from './pendingChange';

const refusedOf = (error: unknown): Refused =>
  error instanceof Refused ? error : new Refused(0, { refusal: 'Unanswered', reason: String(error) });

/** A refusal by its name and the service's reason. */
export function RefusalLine({ refused }: { refused: Refused }) {
  return <div className="why-not"><b>{refused.refusal.refusal}</b> <span className="sec">{refused.refusal.reason}</span></div>;
}

/** The tab already names the view; this says what it holds. */
function Head({ sub }: { sub: string }) {
  return <p className="sub">{sub}</p>;
}

/* Grants */

export function SecretGrants({ read }: { read: () => Promise<SecretGrantListing> }) {
  const load = useLoad(read, 'secret-grants');
  return <div className="secrets-view">
    <Head sub="Grants on the secrets you can see." />
    <Gate load={load} title="Secret grants" ok={(listing) => <GrantRows listing={listing} />} />
  </div>;
}

const RELATION_WORDS: Record<string, string> = {
  use: 'May use it',
  read: 'May read it',
  lend: 'May lend it on',
  member: 'Stands inside its scope',
};

export function GrantRows({ listing }: { listing: SecretGrantListing }) {
  if (!listing.grants.length) return <p className="note">No grants were returned on the secrets you can see.</p>;
  return <table><thead><tr><th>Secret</th><th>Who</th><th>What they may do</th><th>Granted by</th></tr></thead>
    <tbody>{listing.grants.map((grant) => <tr key={grant.identity + '\u0000' + grant.secret + '\u0000' + grant.relation}>
      <td>{grant.secret}</td><td>{grant.identity}</td><td>{RELATION_WORDS[grant.relation] ?? grant.relation}</td><td>{grant.granted_by ?? 'Not recorded'}</td>
    </tr>)}</tbody>
  </table>;
}

/* Audit */

export function SecretAudit({ read }: { read: () => Promise<SecretAuditLog> }) {
  const load = useLoad(read, 'secret-audit');
  return <div className="secrets-view">
    <Head sub="The broker's checked record of the secrets you can see, newest first." />
    <Gate load={load} title="Secret audit" ok={(log) => <AuditRows log={log} />} />
  </div>;
}

const KIND_WORDS: Record<SecretAuditKind, string> = {
  issue: 'Handle issued',
  use: 'Used',
  drop: 'Handle dropped',
  seal: 'Stored',
  rotation: 'Store key changed',
  next_account: 'Moved to next account',
  settlement: 'Call settled',
  sealed_read: 'Record read',
  refresh: 'Access renewed or withdrawn',
  spawn_login: 'Seat sign-in handed over',
};

/** The log's lines by index, highest first: the broker numbers lines in the order it wrote them. */
export function AuditRows({ log }: { log: SecretAuditLog }) {
  if (!log.lines.length) return <p className="note">Nothing is recorded yet on the secrets you can see.</p>;
  const lines = [...log.lines].sort((a, b) => b.index - a.index);
  return <table><thead><tr><th>#</th><th>When</th><th>What</th><th>Secret</th><th>For</th><th>Handle id</th><th>Uses so far</th><th>Outcome</th></tr></thead>
    <tbody>{lines.map((line) => <tr key={line.index}>
      <td>{line.index}</td><td>{clock(Math.floor(line.at_ms / 1000))}</td><td>{KIND_WORDS[line.kind] ?? line.kind}</td>
      <td>{line.secret ?? ''}</td><td>{line.identity ?? ''}</td><td>{line.handle ?? ''}</td>
      <td>{line.uses === null ? '' : line.uses}</td><td>{line.outcome}</td>
    </tr>)}</tbody>
  </table>;
}

/* Revocation */

export type RevocationOutcome =
  | { at: 'idle' }
  | { at: 'checking' }
  | { at: 'answered'; answer: RevocationAnswer }
  | { at: 'refused'; refused: Refused };

export function RevocationLookup({ check }: { check: (handle: string) => Promise<RevocationAnswer> }) {
  const [handle, setHandle] = useState('');
  const [outcome, setOutcome] = useState<RevocationOutcome>({ at: 'idle' });
  const asking = useRef(false);
  const submit = async (event: FormEvent) => {
    event.preventDefault();
    const id = handle.trim();
    if (!id || asking.current) return;
    asking.current = true;
    setOutcome({ at: 'checking' });
    try {
      setOutcome({ at: 'answered', answer: await check(id) });
    } catch (error) {
      setOutcome({ at: 'refused', refused: refusedOf(error) });
    } finally {
      asking.current = false;
    }
  };
  return <div className="secrets-view">
    <Head sub="Whether use has stopped here, and whether the provider has been asked to withdraw its access too." />
    <form onSubmit={submit}>
      <label className="field">Handle id<input value={handle} onChange={(event) => setHandle(event.target.value)} placeholder="The id from the audit record" /></label>
      <button className="btn primary" type="submit" disabled={outcome.at === 'checking' || !handle.trim()}>Check</button>
    </form>
    <RevocationView outcome={outcome} />
  </div>;
}

/** The provider's part of a revocation in plain words. */
export function upstreamWords(answer: RevocationAnswer): string {
  switch (answer.upstream) {
    case 'not_asked': return 'not asked';
    case 'unconfirmed': return 'asked, not yet confirmed: ' + (answer.upstream_reason ?? 'no reason was given');
    case 'confirmed': return 'confirmed by the provider';
  }
}

export function RevocationView({ outcome }: { outcome: RevocationOutcome }) {
  switch (outcome.at) {
    case 'idle': return null;
    case 'checking': return <p className="note">Checking…</p>;
    case 'refused':
      if (outcome.refused.refusal.refusal === 'HandleUnknown') return <p className="note">No handle with that id is visible to you.</p>;
      return <RefusalLine refused={outcome.refused} />;
    case 'answered': return <div>
      <p>Handle id: {outcome.answer.handle}</p>
      <p>Use stopped here: {outcome.answer.stopped_here ? 'yes' : 'no'}</p>
      <p>The provider: {upstreamWords(outcome.answer)}</p>
    </div>;
  }
}

/* Owner changes */

export type ChangeOutcome<T> =
  | { at: 'editing' }
  | { at: 'sending' }
  | { at: 'done'; answer: T }
  | { at: 'refused'; refused: Refused }
  | { at: 'unknown'; refused: Refused }
  | { at: 'held'; asked: string };

/** Whether a form in this outcome may send. Only an idle form or a definite refusal may. */
export const maySend = (outcome: ChangeOutcome<unknown>): boolean => outcome.at === 'editing' || outcome.at === 'refused';

const UNKNOWN_NOTE = 'It is not known whether this change was made. Its original request is retained. Retry original change checks that same operation without creating another change. Older requests without an operation id stay held.';

export function ChangeView<T>({ outcome, done }: { outcome: ChangeOutcome<T>; done: (answer: T) => string }) {
  switch (outcome.at) {
    case 'editing': return null;
    case 'sending': return <p className="note">Saving…</p>;
    case 'done': return <p className="note">{done(outcome.answer)}</p>;
    case 'refused': return <RefusalLine refused={outcome.refused} />;
    case 'unknown': return <>
      <RefusalLine refused={outcome.refused} />
      <p className="note">{UNKNOWN_NOTE}</p>
    </>;
    case 'held': return <p className="note" role="status">A change asked earlier ({outcome.asked}) has no confirmed answer. {UNKNOWN_NOTE}</p>;
  }
}

/** The broker's name for a scope asked as a kind and a name. */
export function scopeAsked(kind: ScopeKind, name: string): string {
  return (kind === 'personal' ? 'person' : kind) + '/' + name;
}

/** A scope answer that says exactly what was asked, or an unconfirmed outcome. */
export function confirmScope(answer: unknown, secret: string, scope: string, operation: string): ScopeChanged {
  if (answer && typeof answer === 'object' && 'secret' in answer && 'scope' in answer
    && answer.secret === secret && answer.scope === scope && 'operation' in answer && answer.operation === operation
    && 'repeated' in answer && typeof answer.repeated === 'boolean') return { secret, scope, operation, repeated: answer.repeated };
  throw unconfirmed(`the service answered, but not with ${secret} kept for ${scope}`);
}

/** A recipients answer that says exactly what was asked, or an unconfirmed outcome. */
export function confirmRecipients(answer: unknown, secret: string, recipients: Recipients, operation: string): RecipientsChanged {
  if (answer && typeof answer === 'object' && 'secret' in answer && 'recipients' in answer
    && answer.secret === secret && answer.recipients === recipients && 'operation' in answer && answer.operation === operation
    && 'repeated' in answer && typeof answer.repeated === 'boolean') return { secret, recipients, operation, repeated: answer.repeated };
  throw unconfirmed(`the service answered, but not with ${secret} handed to ${recipients}`);
}

/** The tab's session storage; a page without one cannot record a change, so it cannot send one. */
function tabStore(): PendingStore {
  if (typeof sessionStorage === 'undefined') throw new Error('this page has no session storage to hold a pending change');
  return sessionStorage;
}

/** A scope as the broker names it (`team/<name>`), in plain words. */
export function scopeWords(scope: string): string {
  const [kind, ...rest] = scope.split('/');
  const name = rest.join('/');
  if (kind === 'person') return 'the person ' + name;
  if (kind === 'team') return 'the team ' + name;
  if (kind === 'organisation') return 'the organisation ' + name;
  return scope;
}

const SCOPE_KINDS: readonly ScopeKind[] = ['personal', 'team', 'organisation'];
const RECIPIENTS: readonly Recipients[] = ['anyone', 'people_only'];

/** The option the select offered; a value it never offered is a defect, never a default. */
function offered<T extends string>(options: readonly T[], value: string): T {
  const found = options.find((option) => option === value);
  if (found === undefined) throw new Error(`the form offered no option ${JSON.stringify(value)}`);
  return found;
}
const scopeKindOf = (value: string): ScopeKind => offered(SCOPE_KINDS, value);
const recipientsOf = (value: string): Recipients => offered(RECIPIENTS, value);

const OWNER_NOTE = 'Only the secret\'s owner can change this. Anyone else is refused.';

export function ScopeChange({ change, secret: initial = '', store = tabStore(), changed }: { change: (secret: string, kind: ScopeKind, name: string, operation: string) => Promise<unknown>; secret?: string; store?: PendingStore; changed?: () => void }) {
  const [secret, setSecret] = useState(initial);
  const [kind, setKind] = useState<ScopeKind>('personal');
  const [name, setName] = useState('');
  const { outcome, run, edited, retry } = useOwnerChange<ScopeChanged>(pendingKey('scope', secret.trim()), store, async (asked, operation) => {
    if (asked.type !== 'scope') throw unconfirmed('The retained change is not a scope change.');
    const confirmed = confirmScope(await change(asked.secret, asked.kind, asked.name, operation), asked.secret, scopeAsked(asked.kind, asked.name), operation);
    changed?.();
    return confirmed;
  });
  const ready = secret.trim() !== '' && name.trim() !== '';
  const submit = (event: FormEvent) => {
    event.preventDefault();
    if (!ready) return;
    const asked = { secret: secret.trim(), kind, name: name.trim() };
    const scope = scopeAsked(asked.kind, asked.name);
    run(`${asked.secret} kept for ${scope}`, { type: 'scope', ...asked });
  };
  return <form onSubmit={submit}>
    <h2>Who can see this secret</h2>
    <p className="note">{OWNER_NOTE}</p>
    <label className="field">Secret<input disabled={outcome.at !== 'editing' && outcome.at !== 'refused' && outcome.at !== 'done'} value={secret} onChange={(event) => { edited(); setSecret(event.target.value); }} /></label>
    <label className="field">Kept for<select disabled={outcome.at !== 'editing' && outcome.at !== 'refused' && outcome.at !== 'done'} value={kind} onChange={(event) => { edited(); setKind(scopeKindOf(event.target.value)); }}>
      <option value="personal">One person</option><option value="team">A team</option><option value="organisation">An organisation</option>
    </select></label>
    <label className="field">Name<input disabled={outcome.at !== 'editing' && outcome.at !== 'refused' && outcome.at !== 'done'} value={name} onChange={(event) => { edited(); setName(event.target.value); }} placeholder={kind === 'personal' ? 'The person\'s id' : 'Its name'} /></label>
    <button className="btn primary" type="submit" disabled={!maySend(outcome) || !ready}>Save</button>
    {retry ? <button className="btn" type="button" onClick={retry}>Retry original change</button> : null}
    <ChangeView outcome={outcome} done={(answer) => `${answer.secret}: confirmed change to ${scopeWords(answer.scope)}.`} />
  </form>;
}

const RECIPIENT_WORDS: Record<Recipients, string> = {
  anyone: 'anyone who is permitted',
  people_only: 'people only, never agents',
};

export function RecipientsChange({ change, secret: initial = '', store = tabStore(), changed }: { change: (secret: string, recipients: Recipients, operation: string) => Promise<unknown>; secret?: string; store?: PendingStore; changed?: () => void }) {
  const [secret, setSecret] = useState(initial);
  const [recipients, setRecipients] = useState<Recipients>('people_only');
  const { outcome, run, edited, retry } = useOwnerChange<RecipientsChanged>(pendingKey('recipients', secret.trim()), store, async (asked, operation) => {
    if (asked.type !== 'recipients') throw unconfirmed('The retained change is not a recipients change.');
    const confirmed = confirmRecipients(await change(asked.secret, asked.recipients, operation), asked.secret, asked.recipients, operation);
    changed?.();
    return confirmed;
  });
  const ready = secret.trim() !== '';
  const submit = (event: FormEvent) => {
    event.preventDefault();
    if (!ready) return;
    const asked = { secret: secret.trim(), recipients };
    run(`${asked.secret} handed to ${RECIPIENT_WORDS[asked.recipients]}`, { type: 'recipients', ...asked });
  };
  return <form onSubmit={submit}>
    <h2>Who it can be handed to</h2>
    <p className="note">{OWNER_NOTE}</p>
    <label className="field">Secret<input disabled={outcome.at !== 'editing' && outcome.at !== 'refused' && outcome.at !== 'done'} value={secret} onChange={(event) => { edited(); setSecret(event.target.value); }} /></label>
    <label className="field">Can be handed to<select disabled={outcome.at !== 'editing' && outcome.at !== 'refused' && outcome.at !== 'done'} value={recipients} onChange={(event) => { edited(); setRecipients(recipientsOf(event.target.value)); }}>
      <option value="anyone">Anyone who is permitted</option><option value="people_only">People only</option>
    </select></label>
    <button className="btn primary" type="submit" disabled={!maySend(outcome) || !ready}>Save</button>
    {retry ? <button className="btn" type="button" onClick={retry}>Retry original change</button> : null}
    <ChangeView outcome={outcome} done={(answer) => `${answer.secret}: confirmed change to ${RECIPIENT_WORDS[answer.recipients]}.`} />
  </form>;
}
