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

const refusedOf = (error: unknown): Refused =>
  error instanceof Refused ? error : new Refused(0, { refusal: 'Unanswered', reason: String(error) });

/** A refusal by its name and the service's reason. */
export function RefusalLine({ refused }: { refused: Refused }) {
  return <div className="why-not"><b>{refused.refusal.refusal}</b> <span className="sec">{refused.refusal.reason}</span></div>;
}

function Head({ title, sub, busy, refresh }: { title: string; sub: string; busy: boolean; refresh: () => void }) {
  return <div className="head"><div><div className="eyebrow">Secrets</div><h1>{title}</h1><p className="sub">{sub}</p></div>
    <button className="btn" disabled={busy} onClick={refresh}>Refresh</button>
  </div>;
}

/* Grants */

export function SecretGrants({ read }: { read: () => Promise<SecretGrantListing> }) {
  const [revision, setRevision] = useState(0);
  const load = useLoad(read, 'secret-grants:' + revision);
  return <div className="page">
    <Head title="Who may use what" sub="Grants on the secrets you can see." busy={load.status === 'loading'} refresh={() => setRevision((value) => value + 1)} />
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
  const [revision, setRevision] = useState(0);
  const load = useLoad(read, 'secret-audit:' + revision);
  return <div className="page">
    <Head title="What happened" sub="The broker's checked record of the secrets you can see, newest first." busy={load.status === 'loading'} refresh={() => setRevision((value) => value + 1)} />
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
  return <div className="page">
    <div className="head"><div><div className="eyebrow">Secrets</div><h1>Has a handle been stopped?</h1>
      <p className="sub">Whether use has stopped here, and whether the provider has been asked to withdraw its access too.</p></div></div>
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
  | { at: 'unknown'; refused: Refused };

/** Run one change, once. A change whose outcome is unknown is never repeated by itself. */
function useChange<T>() {
  const [outcome, setOutcome] = useState<ChangeOutcome<T>>({ at: 'editing' });
  const sending = useRef(false);
  const run = async (send: () => Promise<T>) => {
    if (sending.current) return;
    sending.current = true;
    setOutcome({ at: 'sending' });
    try {
      setOutcome({ at: 'done', answer: await send() });
    } catch (error) {
      const refused = refusedOf(error);
      setOutcome(refused.status === 0 || refused.status >= 500 ? { at: 'unknown', refused } : { at: 'refused', refused });
    } finally {
      sending.current = false;
    }
  };
  return { outcome, run };
}

export function ChangeView<T>({ outcome, done }: { outcome: ChangeOutcome<T>; done: (answer: T) => string }) {
  switch (outcome.at) {
    case 'editing': return null;
    case 'sending': return <p className="note">Saving…</p>;
    case 'done': return <p className="note">{done(outcome.answer)}</p>;
    case 'refused': return <RefusalLine refused={outcome.refused} />;
    case 'unknown': return <>
      <RefusalLine refused={outcome.refused} />
      <p className="note">It is not known whether this change was made. Check the secret before asking again.</p>
    </>;
  }
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

export function ScopeChange({ change, secret: initial = '' }: { change: (secret: string, kind: ScopeKind, name: string) => Promise<ScopeChanged>; secret?: string }) {
  const [secret, setSecret] = useState(initial);
  const [kind, setKind] = useState<ScopeKind>('personal');
  const [name, setName] = useState('');
  const { outcome, run } = useChange<ScopeChanged>();
  const ready = secret.trim() !== '' && name.trim() !== '';
  const submit = (event: FormEvent) => {
    event.preventDefault();
    if (ready) void run(() => change(secret.trim(), kind, name.trim()));
  };
  return <form onSubmit={submit}>
    <h2>Who can see this secret</h2>
    <p className="note">{OWNER_NOTE}</p>
    <label className="field">Secret<input value={secret} onChange={(event) => setSecret(event.target.value)} /></label>
    <label className="field">Kept for<select value={kind} onChange={(event) => setKind(scopeKindOf(event.target.value))}>
      <option value="personal">One person</option><option value="team">A team</option><option value="organisation">An organisation</option>
    </select></label>
    <label className="field">Name<input value={name} onChange={(event) => setName(event.target.value)} placeholder={kind === 'personal' ? 'The person\'s id' : 'Its name'} /></label>
    <button className="btn primary" type="submit" disabled={outcome.at === 'sending' || !ready}>Save</button>
    <ChangeView outcome={outcome} done={(answer) => `${answer.secret} is now kept for ${scopeWords(answer.scope)}.`} />
  </form>;
}

const RECIPIENT_WORDS: Record<Recipients, string> = {
  anyone: 'anyone who is permitted',
  people_only: 'people only, never agents',
};

export function RecipientsChange({ change, secret: initial = '' }: { change: (secret: string, recipients: Recipients) => Promise<RecipientsChanged>; secret?: string }) {
  const [secret, setSecret] = useState(initial);
  const [recipients, setRecipients] = useState<Recipients>('people_only');
  const { outcome, run } = useChange<RecipientsChanged>();
  const ready = secret.trim() !== '';
  const submit = (event: FormEvent) => {
    event.preventDefault();
    if (ready) void run(() => change(secret.trim(), recipients));
  };
  return <form onSubmit={submit}>
    <h2>Who it can be handed to</h2>
    <p className="note">{OWNER_NOTE}</p>
    <label className="field">Secret<input value={secret} onChange={(event) => setSecret(event.target.value)} /></label>
    <label className="field">Can be handed to<select value={recipients} onChange={(event) => setRecipients(recipientsOf(event.target.value))}>
      <option value="anyone">Anyone who is permitted</option><option value="people_only">People only</option>
    </select></label>
    <button className="btn primary" type="submit" disabled={outcome.at === 'sending' || !ready}>Save</button>
    <ChangeView outcome={outcome} done={(answer) => `${answer.secret} can now be handed to ${RECIPIENT_WORDS[answer.recipients]}.`} />
  </form>;
}
