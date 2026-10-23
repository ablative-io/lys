import { useEffect, useMemo, useState } from 'react';
import type { FormEvent } from 'react';
import { Refused, api, operationId, request, send, useLoad } from '../../api';
import type { MeView, PeopleView } from '../../generated';
import { Gate, RefusedPage } from '../signin/Gate';
import { PageHeader } from '../../shell/PageHeader';
import { AgentGrantPicker } from '../people/AgentGrantPicker';
import type { GrantGroup } from '../people/AgentGrantPicker';
import { grantOptionKey, grantOptions, newAgentGrants, readGrantChoices } from '../people/agent-grants';
import type { GrantChoices } from '../people/agent-grants';
import { resourceNames, resourceWords } from '../people/agent-resource-names';
import { withheldFromAgents } from '../grants/ActionPicker';
import { ErrorWords } from '../people/Words';
import { readTeams } from '../teams/Teams';
import type { Team } from '../teams/contract';

/**
 * The Pair page (AGENTS-020 R2): an agent with no identity said four words; the person enters them here, names
 * the agent, chooses what it is given, and approves or declines. The words typed are kept in this tab's storage,
 * never in the address, so a stranger sent to sign in comes back to them (keptCode).
 */
export const PAIR_CODE = 'lys.pair.code';

export function keptCode(): string {
  try { return sessionStorage.getItem(PAIR_CODE) ?? ''; } catch { return ''; }
}
function keepCode(code: string): void {
  try {
    if (code) sessionStorage.setItem(PAIR_CODE, code);
    else sessionStorage.removeItem(PAIR_CODE);
  } catch { /* No storage in this browser: the words live in the field alone. */ }
}
/** The four words as the service keys them: lowercase, single spaces. */
export const normalise = (typed: string): string => typed.trim().toLowerCase().split(/\s+/).filter(Boolean).join(' ');

interface Waiting { code: string; name: string; asked_at: number }
type Found =
  | { state: 'none' }
  | { state: 'unknown' }
  | { state: 'waiting'; waiting: Waiting }
  | { state: 'refused'; refused: unknown }
  | { state: 'paired'; name: string }
  | { state: 'declined' };

const waitingPath = (code: string) => '/pair/' + encodeURIComponent(code);

export function PairPage() {
  const [code, setCode] = useState(keptCode);
  const [found, setFound] = useState<Found>({ state: 'none' });
  const [busy, setBusy] = useState(false);
  useEffect(() => { keepCode(code); }, [code]);

  async function lookUp(typed: string) {
    const words = normalise(typed);
    if (!words || busy) return;
    setBusy(true);
    try {
      const waiting = await send<Waiting>('GET', waitingPath(words) + '/waiting');
      setFound({ state: 'waiting', waiting });
    } catch (problem) {
      if (problem instanceof Refused && problem.status === 404) setFound({ state: 'unknown' });
      else setFound({ state: 'refused', refused: problem });
    } finally {
      setBusy(false);
    }
  }
  // Back from sign-in with the words kept: they are looked up again without retyping.
  useEffect(() => { const kept = keptCode(); if (kept) void lookUp(kept); }, []); // eslint-disable-line react-hooks/exhaustive-deps

  function submit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    void lookUp(code);
  }
  const decided = (outcome: Found) => { setFound(outcome); setCode(''); };

  return (
    <div className="page">
      <PageHeader title="Pair an agent" subtitle="An agent with no identity yet says four words. Enter them, name the agent, choose what it is given, and approve." />
      <form onSubmit={submit} aria-label="The agent's four words" className="pair-words">
        <label>The four words the agent said
          <input name="code" value={code} onChange={(event) => setCode(event.target.value)} autoComplete="off" spellCheck={false} placeholder="four words, with spaces" />
        </label>
        <button type="submit" className="btn" disabled={busy || !normalise(code)}>Find the agent</button>
      </form>
      {found.state === 'unknown' ? <p role="status" data-pair-unknown>No agent is waiting with these words.</p> : null}
      {found.state === 'refused' ? (found.refused instanceof Refused ? <RefusedPage refused={found.refused} title="Pair an agent" /> : <p role="alert">{String(found.refused)}</p>) : null}
      {found.state === 'waiting' ? <Decide waiting={found.waiting} decided={decided} /> : null}
      {found.state === 'paired' ? <p role="status" data-pair-paired>Paired: {found.name} can now act as itself.</p> : null}
      {found.state === 'declined' ? <p role="status" data-pair-declined>Declined. Nothing of the agent's key was kept.</p> : null}
    </div>
  );
}

function Decide({ waiting, decided }: { waiting: Waiting; decided: (outcome: Found) => void }) {
  const load = useLoad(() => Promise.all([api.me(), api.people(), readTeams(), readGrantChoices()]), 'pair:' + waiting.code);
  return <Gate load={load} title="Pair an agent" reading="Reading who you are and what you can give…" ok={([me, people, teams, grants]) => <DecideForm me={me} people={people} teams={teams} grants={grants} waiting={waiting} decided={decided} />} />;
}

function DecideForm({ me, people, teams, grants, waiting, decided }: { me: MeView; people: PeopleView; teams: Team[]; grants: GrantChoices; waiting: Waiting; decided: (outcome: Found) => void }) {
  const [name, setName] = useState(waiting.name);
  const [selected, setSelected] = useState<string[]>([]);
  const [problem, setProblem] = useState<unknown>(null);
  const [busy, setBusy] = useState(false);
  const options = useMemo(() => grantOptions(grants, me.person.id, me.person.id, me.person.display_name), [grants, me.person.id, me.person.display_name]);
  const labels = useMemo(() => resourceNames(people, teams, null), [people, teams]);
  const groups = useMemo<GrantGroup[]>(() => {
    const byResource = new Map<string, GrantGroup>();
    for (const option of options) {
      const resource = option.grant.resource;
      const id = JSON.stringify(resource);
      const label = resourceWords(resource, labels);
      const group = byResource.get(id) ?? { id, title: label.words, href: label.unnamed ? '#/resources' : undefined, choices: [] };
      group.choices.push({ id: grantOptionKey(option), resource, actions: option.actions, reason: option.reason });
      byResource.set(id, group);
    }
    return [...byResource.values()];
  }, [options, labels]);
  const pickable = grants.model !== null && withheldFromAgents(grants.model) !== null;

  async function approve(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (busy || !name.trim()) return;
    setBusy(true);
    setProblem(null);
    try {
      const roots = newAgentGrants(options, selected).map((grant) => ({ operation: grant.operation, source: grant.source, resource: grant.resource, relation: grant.relation, window: grant.window }));
      await request(waitingPath(waiting.code) + '/approve', { operation: operationId(), activate: operationId(), certificate: operationId(), display_name: name.trim(), grants: roots });
      decided({ state: 'paired', name: name.trim() });
    } catch (failure) {
      setProblem(failure);
    } finally {
      setBusy(false);
    }
  }
  async function decline() {
    if (busy) return;
    setBusy(true);
    setProblem(null);
    try {
      await request(waitingPath(waiting.code) + '/decline', {});
      decided({ state: 'declined' });
    } catch (failure) {
      setProblem(failure);
    } finally {
      setBusy(false);
    }
  }

  return (
    <form onSubmit={approve} aria-label="Pair this agent" className="pair-decide">
      <p>An agent is waiting with these words. It proposed the name <strong>{waiting.name}</strong>.</p>
      <label>Its name
        <input name="display_name" value={name} onChange={(event) => setName(event.target.value)} autoComplete="off" />
      </label>
      {pickable && grants.model ? <AgentGrantPicker model={grants.model} groups={groups} selected={selected} change={setSelected} disabled={busy} /> : null}
      {grants.problem ? <ErrorWords problem={grants.problem} /> : null}
      <p className="pair-actions">
        <button type="submit" className="btn" disabled={busy || !name.trim()}>Approve</button>
        <button type="button" className="btn" onClick={() => { void decline(); }} disabled={busy}>Decline</button>
      </p>
      {problem ? <ErrorWords problem={problem} /> : null}
    </form>
  );
}
