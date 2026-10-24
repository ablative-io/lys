import { useContext, useMemo, useState } from 'react';
import { Refused, api, operationId, request, useLoad } from '../../api';
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
import type { Machine, NetworkView } from '../network/contract';
import { NetworkRead } from '../network/shared-read';
import type { Entry } from '../people/directory';
import { actionWords } from '../grants/action-words';
import { ORGANISATION_KIND, relationsOf } from '../grants/IssueRoot';
import { organisationActs } from '../grants/organisation-acts';
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

/** One grant the standing choice posts: the act, the relation carrying it alone, and the held grant it is given from. */
interface Step { action: string; relation: string; source: Grant }

/** A plan for one act as drawn: its step when the person can give it, else the words for why not. */
type Plan = { at: 'ready'; steps: Step[] } | { at: 'none'; why: string };

type Outcome =
  | { at: 'idle' }
  | { at: 'sending' }
  | { at: 'given'; words: string }
  | { at: 'refused'; words: string };

/**
 * The five grants of the one standing choice, in the order they are given (AGENTS-017 R4; the fifth, issuing the
 * person's agents' certificates, since 24 October 2026).
 */
const ACTS: { action: string; on: 'person' | 'computer' }[] = [
  { action: 'agent.create', on: 'person' },
  { action: 'machine.agent.set', on: 'computer' },
  { action: 'agent.provisioning.set', on: 'person' },
  { action: 'agent.provisioning.review', on: 'person' },
  { action: 'agent.certificate.issue', on: 'person' },
];

/**
 * The person's grants a standing choice is given from, one per act: their own, standing, passable to an agent, on
 * exactly `resource` (POST /grants gives only on its source's resource), carrying the act alone as the model's
 * `only.<action>` relation names it. An act the model withholds from agents, or one the person holds nothing to give
 * of, is said, never offered.
 */
function planFor(w: GrantWorld, resource: ResourceRef | null, actions: string[]): Plan {
  if (resource === null) return { at: 'none', why: 'You have no computer of your own to give.' };
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

/** The person's own computers in use, from a read of the network. */
const ownComputers = (network: NetworkView | null, me: string): Machine[] =>
  network && Array.isArray(network.machines)
    ? network.machines.filter((machine) => readableMachine(machine) && machine.named_by === me && machine.state === 'in_use')
    : [];

/** The computer the choice gives on: the one chosen, else the one Lys runs on, else the first. */
const chosenOf = (computers: Machine[], chosen: string): Machine | undefined =>
  computers.find((machine) => machine.id === chosen) ?? computers.find((machine) => machine.runtime === 'lys-runner') ?? computers[0];

/** Each act's plan, in order, on the person and the computer. */
const plansFor = (w: GrantWorld, computer: Machine | undefined): { action: string; plan: Plan }[] =>
  ACTS.map(({ action, on }) => ({
    action,
    plan: planFor(w, on === 'person' ? { kind: 'person', id: w.me.person.id } : computer ? { kind: 'machine', id: computer.id } : null, [action]),
  }));

/**
 * The one standing choice a person gives an agent once (AGENTS-015 R4, AGENTS-017 R4): let it set up and run agents
 * on a computer of theirs. Its Give posts five grants in order, agent.create, machine.agent.set,
 * agent.provisioning.set, agent.provisioning.review and agent.certificate.issue, through POST /grants exactly as the
 * pickers below post theirs:
 * same body, same kept request, same confirmation, use only. The administrator who holds nothing to give one of them
 * from has Lys record their roots first (POST /grants/agent-roots), which also names the computer Lys runs on; the
 * grants and computers are read again before the five are planned. One receipt names the five grants; a refusal names
 * the step that refused and leaves the earlier grants given, so pressing Give again finishes what is left.
 */
function StandingGrants({ x, w, reload }: { x: Entry; w: GrantWorld; reload: () => void }) {
  const me = w.me.person.id;
  const name = x.display_name;
  const administrator = w.people.scope === 'directory';
  const readNetwork = useContext(NetworkRead);
  const network = useLoad(readNetwork, 'standing-computers:' + x.id);
  const [computer, setComputer] = useState('');
  const [outcome, setOutcome] = useState<Outcome>({ at: 'idle' });
  const computers = ownComputers(network.status === 'ok' ? network.data : null, me);
  const chosen = chosenOf(computers, computer);
  const plans = plansFor(w, chosen);
  const missing = plans.find(({ plan }) => plan.at === 'none');
  const where = chosen?.name ?? 'a computer of yours';
  const title = `Let ${name} set up and run agents on ${chosen?.name ?? 'your computer'}`;
  const sentence = `Lets ${name} register agents that answer to you; it then may set up, start, stop and restart each agent it creates. `
    + `Lets ${name} allow your agents to run on ${where}. `
    + `Lets ${name} set up and review your agents’ profiles; a version ${name} sets itself is reviewed by someone else. `
    + `Lets ${name} issue your agents’ certificates, the keys they act with, so a new or re-keyed agent needs no sign-in of yours.`;

  /** The roots first when the administrator lacks a source, then each step's grant in turn, kept before it is sent. */
  const give = async () => {
    if (outcome.at === 'sending') return;
    setOutcome({ at: 'sending' });
    const given: string[] = [];
    let world = w;
    let on = chosen;
    if (missing && administrator) {
      try {
        await request<unknown>('/grants/agent-roots', {});
        const [list, read] = await Promise.all([api.grants(), readNetwork()]);
        world = { ...w, list, byId: new Map(list.grants.map((grant) => [grant.id, grant])) };
        on = chosenOf(ownComputers(read, me), computer);
      } catch (error) {
        const reason = error instanceof Refused ? error.refusal.reason : String(error);
        setOutcome({ at: 'refused', words: `Not given: ${title}. Lys refused to record your roots for giving agents access: ${reason}.` });
        return;
      }
    }
    const now = Math.floor(Date.now() / 1000);
    for (const { action, plan } of plansFor(world, on)) {
      const before = given.length ? ` Given before it: ${given.join(', ')}.` : '';
      if (plan.at === 'none') {
        setOutcome({ at: 'refused', words: `Not given: ${title}. ${action}: ${plan.why}${before}` });
        if (given.length) reload();
        return;
      }
      for (const step of plan.steps) {
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
          setOutcome({ at: 'refused', words: kept.at === 'refused' ? `Not given: ${title}. Lys refused ${step.action}: ${reason}.${before}`
            : kept.at === 'unkept' ? `Nothing more was sent: this browser could not keep the request for ${step.action} (${reason}).${before}`
              : `Lys has not confirmed ${step.action} (${reason}). The request is kept in this browser; choose its access under Access to give below to check it, never giving it twice.${before}` });
          if (given.length) reload();
          return;
        }
        given.push(`${step.action} (${grantNo(kept.answer.grant)})`);
      }
    }
    setOutcome({ at: 'given', words: `Given: ${title}. ${sentence} ${given.join(', ')}.` });
    reload();
  };

  const reading = network.status === 'loading';
  const offer = !reading && (missing === undefined || administrator);
  return (
    <div className="field" data-standing>
      <div className="eyebrow">Give once</div>
      <p className="note">
        This is one choice, given once. Six approvals for every agent is security too awkward to use, so the standing grant is your
        consent, and every act under it is on the record as {name}’s. Each grant can be revoked from the grants below.
      </p>
      <div className="field" data-standing-choice="all">
        <strong>{title}</strong>
        <p className="note">{sentence}</p>
        {computers.length > 1 ? (
          <select aria-label="Computer to give" value={chosen?.id ?? ''} onChange={(event) => setComputer(event.target.value)}>
            {computers.map((machine) => <option key={machine.id} value={machine.id}>{machine.name}</option>)}
          </select>
        ) : null}
        {reading ? <p className="note" data-standing-reading>Reading your computers…</p>
          : network.status === 'refused' ? <p className="note">Your computers could not be read: {network.refused.refusal.reason}</p> : null}
        {offer ? (
          <Act symbol="send" name={title} word="Give" data-act="standing-all" disabled={outcome.at === 'sending'}
            onClick={() => { void give(); }} />
        ) : !reading && missing?.plan.at === 'none' ? <p className="note">{missing.action}: {missing.plan.why}</p> : null}
      </div>
      {outcome.at === 'given' ? <p role="status" data-standing-receipt>{outcome.words}</p> : null}
      {outcome.at === 'refused' ? <p role="alert" data-standing-refused>{outcome.words}</p> : null}
    </div>
  );
}

/**
 * Standing in the organisation itself, given to an agent by whoever holds organisation.grant there (24 October 2026):
 * the same POST /grants from administrator standing the Give access form posts, from the agent's own file, so the two
 * places agree. The server judges it; this offers only the organisation's standing relations, administrator among them.
 */
function OrganisationStanding({ x, w, reload }: { x: Entry; w: GrantWorld; reload: () => void }) {
  const acts = organisationActs(w.me, w.list.grants);
  const [relation, setRelation] = useState('');
  const [outcome, setOutcome] = useState<Outcome>({ at: 'idle' });
  if (!acts.organisation || !acts.grant) return null;
  const organisation = acts.organisation;
  const relations = relationsOf(w.model, ORGANISATION_KIND);
  const resource: ResourceRef = { kind: ORGANISATION_KIND, id: organisation.id };
  const said = `${relation} of ${organisation.display_name} to ${x.display_name}`;
  const give = async () => {
    const actions = relations[relation];
    if (!actions || outcome.at === 'sending') return;
    setOutcome({ at: 'sending' });
    const me = w.me.person.id;
    const body: DelegateBody = {
      operation: operationId(), route: 'browser', source: 'administrator', recipient: x.id, responsible: me, mode: 'outright',
      resource, relation, pass_on: { kind: 'use_only' }, window: { starts_at: Math.floor(Date.now() / 1000), ends_at: null },
    };
    const kept = await keptSend<Recorded>(pendingGrantKey(me, 'administrator.' + organisation.id), body, async () => {
      const answer = await api.delegate(body);
      if (answer.operation !== body.operation || answer.receipt?.caller !== me || typeof answer.grant !== 'string' || !/^grant-[0-9a-f]{32}$/.test(answer.grant)) {
        throw new Refused(200, { refusal: 'UnconfirmedAnswer', reason: 'Lys did not confirm this grant operation.' });
      }
      return answer;
    }, true);
    if (kept.at !== 'confirmed') {
      const reason = kept.error instanceof Refused ? kept.error.refusal.reason : String(kept.error);
      setOutcome({ at: 'refused', words: kept.at === 'refused' ? `Not given: Lys refused ${said}: ${reason}.`
        : kept.at === 'unkept' ? `Nothing was sent: this browser could not keep the request for ${said} (${reason}).`
          : `Lys has not confirmed ${said} (${reason}). The request is kept in this browser.` });
      return;
    }
    setOutcome({ at: 'given', words: `Given: ${x.display_name} is now ${relation} of ${organisation.display_name} (${grantNo(kept.answer.grant)}). ${actionWords(w.model, resource, actions)}.` });
    reload();
  };
  return (
    <div className="field" data-organisation-standing>
      <div className="eyebrow">In {organisation.display_name}</div>
      <p className="note">
        Standing in the organisation itself, given from your own standing there. An administrator adds people, gives access
        directly and administers the organisation; every act under it is on the record as {x.display_name}’s.
      </p>
      <label className="field">
        Standing to give
        <select aria-label="Standing in the organisation" value={relation} onChange={(event) => setRelation(event.target.value)}>
          <option value="">Choose standing</option>
          {Object.entries(relations).map(([name, actions]) => (
            <option key={name} value={name}>{name} · {actionWords(w.model, resource, actions)}</option>
          ))}
        </select>
      </label>
      <Act symbol="send" name={`Give ${x.display_name} standing in ${organisation.display_name}`} word="Give" data-act="organisation-standing"
        disabled={!relation || outcome.at === 'sending'} onClick={() => { void give(); }} />
      {outcome.at === 'given' ? <p role="status" data-organisation-receipt>{outcome.words}</p> : null}
      {outcome.at === 'refused' ? <p role="alert" data-organisation-refused>{outcome.words}</p> : null}
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
  // Whoever holds organisation.grant gives an agent standing in the organisation outright, holding nothing passable.
  const standing = w.people.scope === 'directory' || organisationActs(w.me, w.list.grants).grant;
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
              if (mineToGive && (passable.length || (x.kind === 'agent' && standing))) {
                setSourceId('');
                setFromLink(false);
                setOpener(event.currentTarget);
                setGiving(true);
              } else
                shell.toast(
                  x.kind === 'person'
                    ? 'You hold no access you may pass on to people. Ask for it on Requests, or ask whoever gave you yours to let it be passed on.'
                    : mineToGive
                      ? 'You hold no access you can give an agent. Ask for it on Requests, or ask whoever gave you yours to let it be passed on to agents.'
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
              {x.kind === 'agent' && mineToGive ? <><StandingGrants x={x} w={w} reload={reload} /><OrganisationStanding x={x} w={w} reload={reload} /></> : null}
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
