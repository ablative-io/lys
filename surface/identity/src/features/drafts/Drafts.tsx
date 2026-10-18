/**
 * Drafts under Access: what an agent prepared for its responsible person to decide, one table, oldest first. A row
 * opens the prepared request whole under it. A waiting draft is approved or refused in its own row; a decision is
 * kept in this tab until its answer is known, so one whose answer is lost is asked about, never sent twice.
 */
import { Fragment, useEffect, useRef, useState } from 'react';
import { useLocation } from 'react-router';
import { Refused, api, operationId, useLive } from '../../api';
import type { PeopleView } from '../../generated';
import type { GrantModel } from '../../generated/grants';
import { refreshLive } from '../../live';
import { readTogether } from '../../reads';
import { keyable } from '../../shell/keyable';
import { AccessLayout } from '../access/AccessLayout';
import { clock } from '../file/time';
import { actionSentence } from '../grants/action-words';
import { ChangeStatus } from '../roles/ChangeStatus';
import { useRoleChange } from '../roles/useRoleChange';
import { Gate } from '../signin/Gate';
import { ErrorWords } from '../people/Words';
import { confirms, readDrafts } from './contract';
import { ProductDrafts } from './ProductDrafts';
import type { Draft, DraftAnswer, DraftFilter } from './contract';
import './drafts.css';
import { APPROVE, DECLINE, DRAFT_STATE } from '../requests/decision-words';
import { Act } from '../../shell/Act';

const COLUMNS = 5;
const RAW_ID = /^[a-z]+-[0-9a-f]{32}$/;

async function readPage(show: DraftFilter) {
  return readTogether({
    drafts: readDrafts(show), me: api.me(), people: api.people(),
    // The words for an action come from the model; when it cannot be read the page says so and still lists the drafts.
    model: api.model().catch((problem: unknown) => problem instanceof Refused ? problem : new Refused(0, { refusal: 'ModelUnreadable', reason: String(problem) })),
  });
}

/** What the draft would do, in plain words: the action's sentence and what it is on, by name where it is someone, never by an identifier. */
function wantsTo(draft: Draft, model: GrantModel | Refused, names: Map<string, string>): string {
  const { kind, id, action } = draft.target;
  const sentence = model instanceof Refused ? action : actionSentence(model, { kind, id }, action);
  const on = names.get(id) ?? (RAW_ID.test(id) ? 'one ' + kind + ' outside your view' : kind + ' ' + id);
  return sentence + ' · ' + on;
}

/** The agent that prepared a draft, by name; one the directory no longer names is never shown by its identifier. */
const agentOf = (draft: Draft): string => draft.agent?.display_name ?? 'An agent outside your view';

/** Who decided: the person by name, else the sign-in's account name when it is a name, never an identifier. */
const deciderOf = (decided: NonNullable<Draft['decided']>): string =>
  decided.by?.display_name ?? (RAW_ID.test(decided.login.subject) || !/[A-Za-z]/.test(decided.login.subject) || /^[0-9a-f-]{20,}$/i.test(decided.login.subject) ? 'someone outside your view' : decided.login.subject);

function Decided({ draft, listed }: { draft: Draft; listed: (id: string) => boolean }) {
  const said = DRAFT_STATE[draft.state] ?? draft.state;
  const replacement = draft.decided?.replacement ?? null;
  return <span className="sec" role={draft.state === 'approved' ? 'alert' : undefined}>{said}{draft.decided ? ' by ' + deciderOf(draft.decided) + ', ' + clock(draft.decided.at) : ''}
    {draft.execution?.state === 'executed' && <span className="draft-reason"><a href={'/api/receipts/' + draft.execution.receipt_index}>View receipt</a></span>}
    {draft.execution?.state === 'refused_on_execution' && <span className="draft-reason">{draft.execution.refusal}: {draft.execution.reason}</span>}
    {draft.decided?.reason ? <span className="draft-reason">{draft.decided.reason}</span> : null}
    {replacement ? <span className="draft-reason">{listed(replacement)
      ? <>Replaced by <a href={'#/access/drafts?draft=' + encodeURIComponent(replacement)} data-act="replacement">the corrected draft</a>.</>
      : 'Replaced by a corrected draft.'}</span> : null}</span>;
}

/** Approve and Decline in the draft's own row. Decline asks for the reason where it stands. Each is a kept change. */
function Decide({ draft, person, changed }: { draft: Draft; person: string; changed: () => void }) {
  const path = '/drafts/' + encodeURIComponent(draft.id);
  const approve = useRoleChange<DraftAnswer>('lys.pending.draft-approve.' + person + '.' + draft.id, path + '/approve', confirms(draft.id), changed);
  const refuse = useRoleChange<DraftAnswer>('lys.pending.draft-refuse.' + person + '.' + draft.id, path + '/refuse', confirms(draft.id), changed);
  const [refusing, setRefusing] = useState(false);
  const [reason, setReason] = useState('');
  const blocked = approve.blocked || refuse.blocked;
  return <div className="draft-decide">
    {refusing && !refuse.pending && !refuse.done
      ? <form aria-label={'Decline the draft of ' + agentOf(draft)} onSubmit={(event) => { event.preventDefault(); if (reason.trim()) refuse.submit({ operation: operationId(), creation_hash: draft.creation_hash, reason: reason.trim() }); }}>
        <label className="field draft-reason-field">Why you decline it<input aria-label="Why you decline it" required value={reason} disabled={blocked} onChange={(event) => setReason(event.target.value)} /><span className="hint">The agent is told this reason.</span></label>
        <Act symbol="decline" name={DECLINE} word={DECLINE} tone="danger" type="submit" data-act="refuse-confirm" disabled={blocked || !reason.trim()} />
        <Act symbol="close" name="Cancel" word="Cancel" data-act="refuse-cancel" disabled={refuse.busy} onClick={() => setRefusing(false)} />
      </form>
      : approve.done || refuse.done || approve.pending || refuse.pending ? null
      : <>
        <Act symbol="approve" name={APPROVE} word={APPROVE} tone="primary" data-act="approve" disabled={blocked} onClick={() => approve.submit({ operation: operationId(), creation_hash: draft.creation_hash, application: operationId() })} />
        <Act symbol="decline" name={DECLINE} word={DECLINE} data-act="refuse" disabled={blocked} onClick={() => setRefusing(true)} />
      </>}
    <ChangeStatus change={approve} />
    <ChangeStatus change={refuse} />
  </div>;
}

function Table({ drafts, show, model, names, person, changed }: { drafts: Draft[]; show: DraftFilter; model: GrantModel | Refused; names: Map<string, string>; person: string; changed: () => void }) {
  const asked = new URLSearchParams(useLocation().search).get('draft');
  const [open, setOpened] = useState<string | null>(asked);
  // A link to a draft in this list opens its row.
  useEffect(() => { if (asked) setOpened(asked); }, [asked]);
  const listed = (id: string) => drafts.some((draft) => draft.id === id);
  const shown = [...drafts].sort((a, b) => a.created_at - b.created_at);
  return <div className="pane">
    <table className="drafts" aria-label="Drafts">
      <thead><tr><th>Agent</th><th>Wants to</th><th>Note</th><th>Asked</th><th>Decision</th></tr></thead>
      <tbody>
        {shown.length ? shown.map((draft) => {
          const isOpen = open === draft.id;
          return <Fragment key={draft.id}>
            <tr data-draft={draft.id} aria-expanded={isOpen} {...keyable(() => setOpened(isOpen ? null : draft.id))}>
              <td>{agentOf(draft)}</td>
              <td>{wantsTo(draft, model, names)}</td>
              <td className="sec draft-note">{draft.note}</td>
              <td className="sec">{clock(draft.created_at)}</td>
              <td onClick={(event) => event.stopPropagation()} onKeyDown={(event) => event.stopPropagation()}>
                {draft.state === 'waiting' ? <Decide draft={draft} person={person} changed={changed} /> : <Decided draft={draft} listed={listed} />}
              </td>
            </tr>
            {isOpen ? <tr className="draft-open"><td colSpan={COLUMNS}>
              <p className="note">The request {draft.agent?.display_name ?? 'the agent'} prepared, exactly as it would be sent:</p>
              <pre className="mono draft-request" aria-label="Prepared request">{draft.method + ' ' + draft.path + '\n\n' + draft.body}</pre>
            </td></tr> : null}
          </Fragment>;
        }) : <tr className="empty"><td colSpan={COLUMNS} className="dim">{show === 'waiting' ? 'Nothing is waiting.' : 'Nothing has been decided yet.'}</td></tr>}
      </tbody>
    </table>
  </div>;
}

const namesOf = (people: PeopleView): Map<string, string> =>
  new Map(people.people.flatMap((person) => [[person.id, person.display_name] as const, ...person.agents.map((agent) => [agent.id, agent.display_name] as const)]));

export function Drafts() {
  const [show, setShow] = useState<DraftFilter>('waiting');
  const [version, setVersion] = useState(0);
  const read = useLive(() => readPage(show), 'drafts:' + show + ':' + version);
  // While the page reads itself again after a decision, what it last showed of the same filter stays; it never blanks.
  const last = useRef<{ show: DraftFilter; read: typeof read } | null>(null);
  if (read.status === 'ok') last.current = { show, read };
  const load = read.status === 'loading' && last.current?.show === show ? last.current.read : read;
  return <AccessLayout on="drafts" className="drafts-page" subtitle="What your agents prepared for you to decide. Nothing is done until you approve it."
    actions={read.status === 'refused' ? <Act symbol="retry" name="Reconnect" word="Reconnect" onClick={refreshLive} /> : null}>
    <div className="tools">
      <div className="seg">{([['waiting', 'Waiting'], ['decided', 'Decided']] as [DraftFilter, string][]).map(([key, label]) => <button key={key} className={show === key ? 'on' : ''} aria-pressed={show === key} onClick={() => setShow(key)}>{label}</button>)}</div>
      {load.status === 'ok' && load.data.model instanceof Refused ? <div className="why-not"><p>What each action means could not be read, so actions are named as the agent wrote them.</p><details><summary>Error details</summary><p className="refusal-name">{load.data.model.refusal.refusal}: {load.data.model.refusal.reason}</p></details></div> : null}
    </div>
    <Gate load={load} title="your drafts" renderError={(error) => <ErrorWords problem={error} />} ok={(data) => <div className="drafts-body">
      <Table drafts={data.drafts} show={show} model={data.model} names={namesOf(data.people)} person={data.me.person.id} changed={() => setVersion((v) => v + 1)} />
      <ProductDrafts names={namesOf(data.people)} person={data.me.person.id} />
    </div>} />
  </AccessLayout>;
}
