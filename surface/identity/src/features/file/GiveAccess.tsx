import { useContext, useMemo, useState } from 'react';
import { Refused, api, operationId, useLoad } from '../../api';
import type { DelegateBody, Grant, Recorded, ResourceRef } from '../../generated/grants';
import { keptSend } from '../../kept';
import { Act } from '../../shell/Act';
import { useShell } from '../../shell/ShellContext';
import { agentMayHold, singleActionCarriers } from '../grants/ActionPicker';
import { Delegate } from '../grants/Delegate';
import { ActPanel } from '../grants/GrantTable';
import { givesTo, grantNo, onText, passesToAgents, withinPassOn, type GrantWorld } from '../grants/model';
import { pendingGrantKey, readPendingGrant, readPendingRest } from '../grants/pendingGrant';
import { readableMachine } from '../network/contract';
import { NetworkRead } from '../network/shared-read';
import type { Entry } from '../people/directory';
import { giveLink } from './giveLink';

/** An unavailable recipient still opens an empty drawer, never an unrelated form. */
export function MissingGive({ id, reason }: { id: string; reason: string }) {
  const [open, setOpen] = useState(true);
  return open ? (
    <ActPanel label="Give" opener={null} close={() => setOpen(false)}>
      <p role="alert">
        This agent does not exist among the records you can see: {id}. {reason} Nothing was filled.
      </p>
      <Act symbol="close" name="Cancel" word="Cancel" onClick={() => setOpen(false)} />
    </ActPanel>
  ) : null;
}

/** One grant a standing choice posts: the act, the relation carrying it alone, and the held grant it is given from. */
interface Step { action: string; relation: string; source: Grant }

/** A standing choice as drawn: its steps when the person can give it, else the words for why not. */
type Plan = { at: 'ready'; steps: Step[] } | { at: 'none'; why: string };

type Standing = 'create' | 'machine' | 'profiles';

type Outcome =
  | { at: 'idle' }
  | { at: 'sending'; which: Standing }
  | { at: 'given'; words: string }
  | { at: 'refused'; words: string };

/**
 * The person's grants a standing choice is given from, one per act: their own, standing, passable to an agent, on
 * exactly `resource` (POST /grants gives only on its source's resource), carrying the act alone as the model's
 * `only.<action>` relation names it. An act the model withholds from agents, or one the person holds nothing to give
 * of, is said, never offered.
 */
function planFor(w: GrantWorld, resource: ResourceRef | null, actions: string[]): Plan {
  if (resource === null) return { at: 'none', why: '' };
  if (!agentMayHold(w.model, resource.kind, actions)) return { at: 'none', why: 'Lys does not let an agent hold this yet.' };
  const carriers = singleActionCarriers(w.model, resource.kind);
  const steps: Step[] = [];
  for (const action of actions) {
    const relation = carriers.get(action);
    const source = w.list.grants.find((grant) => grant.holder === w.me.person.id && grant.standing.stands && passesToAgents(grant.pass_on)
      && grant.resource.kind === resource.kind && grant.resource.id === resource.id && grant.actions.includes(action) && withinPassOn(grant, [action]));
    if (relation === undefined) return { at: 'none', why: 'The permission model has no relation carrying this alone.' };
    if (source === undefined) return { at: 'none', why: 'You hold nothing for this that you may pass on to an agent.' };
    const key = pendingGrantKey(w.me.person.id, source.id);
    if (readPendingGrant(key, w.me.person.id, source.id).kind !== 'empty' || readPendingRest(key, w.me.person.id, source.id).kind !== 'empty') {
      return { at: 'none', why: 'An earlier grant from the same access of yours is not yet confirmed. Choose it under Access to give below to check it first.' };
    }
    steps.push({ action, relation, source });
  }
  return { at: 'ready', steps };
}

/**
 * The three grants a person gives an agent once (AGENTS-015 R4), each one choice in plain words, posted through
 * POST /grants exactly as the pickers below post theirs: same body, same kept request, same confirmation.
 */
function StandingGrants({ x, w, reload }: { x: Entry; w: GrantWorld; reload: () => void }) {
  const me = w.me.person.id;
  const name = x.display_name;
  const readNetwork = useContext(NetworkRead);
  const network = useLoad(readNetwork, 'standing-computers:' + x.id);
  const [computer, setComputer] = useState('');
  const [outcome, setOutcome] = useState<Outcome>({ at: 'idle' });
  const own = network.status === 'ok' && Array.isArray(network.data.machines)
    ? network.data.machines.filter((machine) => readableMachine(machine) && machine.named_by === me && machine.state === 'in_use')
    : [];
  const offered = own.filter((machine) => planFor(w, { kind: 'machine', id: machine.id }, ['machine.agent.set']).at === 'ready');
  const chosen = offered.find((machine) => machine.id === computer) ?? offered[0];
  const choices: { id: Standing; title: string; sentence: string; plan: Plan; absent?: string }[] = [
    { id: 'create', title: 'Create agents for me', plan: planFor(w, { kind: 'person', id: me }, ['agent.create']),
      sentence: `Lets ${name} register agents that answer to you; it then may set up, start, stop and restart each agent it creates.` },
    { id: 'machine', title: 'Run my agents on this computer', plan: planFor(w, chosen ? { kind: 'machine', id: chosen.id } : null, ['machine.agent.set']),
      sentence: chosen ? `Lets ${name} allow your agents to run on ${chosen.name}.` : `Lets ${name} allow your agents to run on a computer of yours.`,
      absent: network.status === 'loading' ? 'Reading your computers…'
        : network.status === 'refused' ? 'Your computers could not be read: ' + network.refused.refusal.reason
          : own.length === 0 ? 'You have no computer of your own to give.'
            : offered.length === 0 ? 'You hold nothing for your computers that you may pass on to an agent.' : undefined },
    { id: 'profiles', title: 'Set up and review this kind of profile', plan: planFor(w, { kind: 'person', id: me }, ['agent.provisioning.set', 'agent.provisioning.review']),
      sentence: `Lets ${name} set up and review your agents’ profiles; a version ${name} sets itself is reviewed by someone else.` },
  ];

  /** Each step's grant in turn, kept before it is sent and released only on Lys's confirmation, as the pickers send theirs. */
  const give = async (which: Standing, title: string, sentence: string, steps: Step[]) => {
    if (outcome.at === 'sending') return;
    setOutcome({ at: 'sending', which });
    const given: string[] = [];
    const now = Math.floor(Date.now() / 1000);
    for (const step of steps) {
      const body: DelegateBody = {
        operation: operationId(), route: 'browser', source: step.source.id, recipient: x.id, responsible: me,
        resource: step.source.resource, relation: step.relation, pass_on: { kind: 'use_only' },
        window: { starts_at: now, ends_at: step.source.window.ends_at },
      };
      const kept = await keptSend<Recorded>(pendingGrantKey(me, step.source.id), body, async () => {
        const answer = await api.delegate(body);
        if (answer.operation !== body.operation || answer.receipt?.caller !== me || typeof answer.grant !== 'string' || !/^grant-[0-9a-f]{32}$/.test(answer.grant)) {
          throw new Refused(200, { refusal: 'UnconfirmedAnswer', reason: 'Lys did not confirm this grant operation.' });
        }
        return answer;
      }, true);
      if (kept.at !== 'confirmed') {
        const reason = kept.error instanceof Refused ? kept.error.refusal.reason : String(kept.error);
        const before = given.length ? ` Given before it: ${given.join(', ')}.` : '';
        setOutcome({ at: 'refused', words: kept.at === 'refused' ? `Not given: ${title}. Lys refused ${step.action}: ${reason}.${before}`
          : kept.at === 'unkept' ? `Nothing more was sent: this browser could not keep the request for ${step.action} (${reason}).${before}`
            : `Lys has not confirmed ${step.action} (${reason}). The request is kept in this browser; choose its access under Access to give below to check it, never giving it twice.${before}` });
        if (given.length) reload();
        return;
      }
      given.push(`${step.action} (${grantNo(kept.answer.grant)})`);
    }
    setOutcome({ at: 'given', words: `Given: ${title}. ${sentence} ${given.join(', ')}.` });
    reload();
  };

  return (
    <div className="field" data-standing>
      <div className="eyebrow">Give once</div>
      <p className="note">
        Each of these is one choice, given once. Six approvals for every agent is security too awkward to use, so the standing grant is your
        consent, and every act under it is on the record as {name}’s. Each can be revoked from the grants below.
      </p>
      {choices.map(({ id, title, sentence, plan, absent }) => (
        <div key={id} className="field" data-standing-choice={id}>
          <strong>{title}</strong>
          {absent === undefined ? <p className="note">{sentence}</p> : null}
          {id === 'machine' && offered.length > 0 ? (
            <select aria-label="Computer to give" value={chosen?.id ?? ''} onChange={(event) => setComputer(event.target.value)}>
              {offered.map((machine) => <option key={machine.id} value={machine.id}>{machine.name}</option>)}
            </select>
          ) : null}
          {absent !== undefined ? <p className="note" {...(network.status === 'loading' ? { 'data-standing-reading': '' } : {})}>{absent}</p>
            : plan.at === 'ready' ? (
              <Act symbol="send" name={title + ' for ' + name} word="Give" data-act={'standing-' + id} disabled={outcome.at === 'sending'}
                onClick={() => { void give(id, title, sentence, plan.steps); }} />
            ) : <p className="note">{plan.why}</p>}
        </div>
      ))}
      {outcome.at === 'given' ? <p role="status" data-standing-receipt>{outcome.words}</p> : null}
      {outcome.at === 'refused' ? <p role="alert" data-standing-refused>{outcome.words}</p> : null}
    </div>
  );
}

/** The same Give form whether opened by a press or filled by a reviewed link. */
export function GiveAccess({ x, w, query, reload }: { x: Entry; w: GrantWorld; query: string; reload: () => void }) {
  const shell = useShell();
  const linked = useMemo(() => giveLink(query, w, x.id), [query, w, x.id]);
  const [giving, setGiving] = useState(linked.at !== 'absent');
  const [opener, setOpener] = useState<HTMLElement | null>(null);
  const [sourceId, setSourceId] = useState(linked.at === 'ready' ? linked.source.id : '');
  const [fromLink, setFromLink] = useState(linked.at !== 'absent');
  const passable = w.list.grants.filter((grant) => grant.holder === w.me.person.id && grant.standing.stands && givesTo(grant.pass_on).includes(x.kind === 'agent' ? 'agent' : 'person'));
  const source = passable.find((grant) => grant.id === sourceId);
  const mineToGive = x.state !== 'retired' && (x.kind === 'person' ? x.id !== w.me.person.id : x.person?.id === w.me.person.id);
  const refused = fromLink && linked.at === 'refused';
  return (
    <>
      <div className="file-section-head">
        <span className="section-h">Grants</span>
        {x.state !== 'retired' && x.id !== w.me.person.id ? (
          <Act
            symbol="send"
            name="Give access"
            word="Give"
            data-act="grant"
            onClick={(event) => {
              if (mineToGive && passable.length) {
                setSourceId('');
                setFromLink(false);
                setOpener(event.currentTarget);
                setGiving(true);
              } else
                shell.toast(
                  x.kind === 'person'
                    ? 'You hold no access you may pass on to people. Ask for it on Requests, or ask whoever gave you yours to let it be passed on.'
                    : mineToGive
                      ? 'You hold no access you can give an agent. The button below records directory access only; Cambium needs a separate grant.'
                      : `Only ${x.display_name}’s responsible person can give it access.`
                );
            }}
          />
        ) : null}
      </div>
      {giving ? (
        <ActPanel label="Give" opener={opener} close={() => setGiving(false)}>
          {fromLink && linked.at === 'ready' ? (
            <p role="status">
              this link filled this form. Recipient: {x.display_name} ({x.id}). Resource kind: {linked.resource.kind}. Id (store): {linked.resource.id}.
              Actions: {linked.actions.join(', ')}. Read the choices below, then press Give.
            </p>
          ) : null}
          {refused ? (
            <>
              <p role="alert">{linked.reason}</p>
              <Act symbol="close" name="Cancel" word="Cancel" onClick={() => setGiving(false)} />
            </>
          ) : (
            <>
              {x.kind === 'agent' && mineToGive ? <StandingGrants x={x} w={w} reload={reload} /> : null}
              <label className="field">
                Access to give
                <select
                  aria-label="Grant to give"
                  value={source?.id ?? ''}
                  onChange={(event) => {
                    setSourceId(event.target.value);
                    setFromLink(false);
                  }}>
                  <option value="">Choose access to give</option>
                  {passable.map((grant) => (
                    <option key={grant.id} value={grant.id}>
                      {onText(grant)} · {grant.relation} · {grantNo(grant.id)}
                    </option>
                  ))}
                </select>
              </label>
              {source ? (
                <Delegate
                  key={source.id}
                  w={w}
                  source={source}
                  to={x.id}
                  initialChoices={fromLink && linked.at === 'ready' ? linked.choices : []}
                  done={reload}
                  close={() => setGiving(false)}
                />
              ) : (
                <p className="hint">Choose a grant to see which actions you can give.</p>
              )}
            </>
          )}
        </ActPanel>
      ) : null}
    </>
  );
}
