import { useMemo, useRef, useState } from 'react';
import type { FormEvent } from 'react';
import { useLocation, useNavigate } from 'react-router';
import { api, useLoad } from '../../api';
import type { MeView, PeopleView } from '../../generated';
import { DirectoryGate as Gate, ErrorWords } from './Words';
import { readTeams } from '../teams/Teams';
import type { Team } from '../teams/contract';
import { addAgent, newAgentRequest, readAgentRequest } from './add-agent-request';
import type { PendingAgent } from './add-agent-request';
import { registrationCapability } from './registration-capability';
import type { RegistrationCapability } from './registration-capability';

function Form({ me, people, teams, search, capability }: { me: MeView; people: PeopleView; teams: Team[]; search: string; capability: RegistrationCapability }) {
  const navigate = useNavigate();
  const key = 'lys.add-agent.' + me.person.id;
  const [saved] = useState(() => {
    try {
      if (sessionStorage.getItem('lys.pending.register-agent') !== null) throw new Error('An earlier registration still needs its outcome checked before adding another agent. Ask the administrator to resolve that saved request.');
      return { pending: readAgentRequest(key), error: null };
    } catch (error) { return { pending: null, error }; }
  });
  const query = new URLSearchParams(search);
  const [name, setName] = useState(saved.pending?.name ?? '');
  const [person, setPerson] = useState(saved.pending ? saved.pending.answersTo ?? me.person.id : query.get('answers_to') ?? me.person.id);
  const [team, setTeam] = useState(saved.pending ? saved.pending.team ?? '' : query.get('team') ?? '');
  const [sending, setSending] = useState(false);
  const [refusal, setRefusal] = useState<unknown>(saved.error);
  const [pending, setPending] = useState(saved.pending);
  const working = useRef(false);
  const peopleById = useMemo(() => new Map(people.people.map((entry) => [entry.id, entry])), [people]);
  const names = useMemo(() => new Map((peopleById.get(person)?.agents ?? []).map((agent) => [agent.display_name.trim().toLowerCase(), agent.display_name])), [peopleById, person]);
  const activeTeams = useMemo(() => teams.filter((entry) => entry.state === 'active'), [teams]);
  const chosen = peopleById.get(person);
  const unavailable = !capability.answersTo && person !== me.person.id;
  const unsupported = unavailable ? pending ? 'Your saved request names another person. This service cannot accept that choice yet; the saved request has not been sent.' : 'Adding an agent under another person is coming. Choose You to add an agent now.' : '';
  const taken = pending ? undefined : names.get(name.trim().toLowerCase());
  const invalidChoice = !chosen || chosen.state !== 'active' ? 'Choose an active person for this agent to answer to.'
    : team && !activeTeams.some((entry) => entry.id === team) ? 'The chosen team is unavailable. Choose an active team or no team.' : '';
  const duplicate = taken ? chosen?.display_name + ' already has an agent named ' + taken + '. Choose another name.' : '';
  const keep = (next: PendingAgent) => { sessionStorage.setItem(key, JSON.stringify(next)); setPending(next); };
  const submit = async (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    if (working.current || saved.error || unavailable || !name.trim() || (!pending && (invalidChoice || taken))) return;
    working.current = true; setSending(true); setRefusal(null);
    try {
      const agent = await addAgent(pending ?? newAgentRequest(name.trim(), person, team), me.person.id, keep, me.signed_in);
      sessionStorage.removeItem(key);
      navigate('/file/' + encodeURIComponent(agent), { replace: true });
    } catch (problem) { setRefusal(problem); }
    finally { working.current = false; setSending(false); }
  };
  const locked = sending || pending !== null || Boolean(saved.error);
  return <form className="add-agent" aria-label="Add an agent" onSubmit={(event) => { void submit(event); }}>
    <label className="field">Name
      <input name="display_name" value={name} autoFocus autoComplete="off" disabled={locked} onChange={(event) => { setName(event.target.value); setRefusal(null); }} aria-invalid={taken ? true : undefined} aria-describedby={taken ? 'add-agent-duplicate' : undefined} />
    </label>
    {duplicate ? <p className="why-not" id="add-agent-duplicate" role="alert">{duplicate}</p> : null}
    <label className="field">Who this agent answers to<select name="answers_to" value={person} disabled={locked} onChange={(event) => setPerson(event.target.value)}>
      {!chosen || chosen.state !== 'active' ? <option value={person}>Choose an active person</option> : null}
      {people.people.filter((entry) => entry.state === 'active').map((entry) => <option key={entry.id} value={entry.id} disabled={!capability.answersTo && entry.id !== me.person.id}>{entry.id === me.person.id ? 'You (' + entry.display_name + ')' : entry.display_name + (!capability.answersTo ? ' (coming)' : '')}</option>)}
    </select></label>
    <label className="field">Team this agent joins<select name="team" value={team} disabled={locked} onChange={(event) => setTeam(event.target.value)}>
      <option value="">No team</option>
      {team && !activeTeams.some((entry) => entry.id === team) ? <option value={team}>Team unavailable</option> : null}
      {activeTeams.map((entry) => <option key={entry.id} value={entry.id}>{entry.name}</option>)}
    </select></label>
    {capability.problem ? <><p>The service’s registration choices could not be read. You can add an agent under You only.</p><details><summary>Registration details</summary><ErrorWords problem={capability.problem} /></details></> : !capability.answersTo ? <p>Adding an agent under another person is coming. You can add an agent under You.</p> : null}
    {unsupported ? <p role="alert" className="why-not">{unsupported}</p> : null}
    {invalidChoice ? <p role="alert" className="why-not">{invalidChoice}</p> : null}
    {refusal ? <ErrorWords problem={refusal} /> : null}
    {pending ? <p role="status">{pending.activated ? 'Your agent was added. Its team membership still needs confirmation.' : pending.agent ? 'Your agent was registered. Activation still needs confirmation.' : 'The registration has no confirmed answer yet.'} The original request is saved. Try that same request again without adding another agent.</p> : null}
    <p><button className="btn primary" type="submit" disabled={sending || unavailable || !name.trim() || Boolean(saved.error) || (!pending && Boolean(taken || invalidChoice))}>{sending ? 'Adding…' : pending ? 'Continue adding this agent' : 'Add agent'}</button></p>
  </form>;
}

export function AddAgent() {
  const location = useLocation();
  const load = useLoad(async () => {
    const [me, people, teams, capability] = await Promise.all([api.me(), api.people(), readTeams(), registrationCapability()]);
    return { me, people, teams, capability };
  }, 'add-agent');
  return <div className="page"><a href="#/people">People and agents</a><h1>Add an agent</h1>
    <Gate load={load} title="Add an agent" ok={(data) => data.people.scope === 'directory' ? <Form key={location.search} me={data.me} people={data.people} teams={data.teams} search={location.search} capability={data.capability} /> : <p className="why-not">Only the Lys administrator can add agents. Ask them to add it for you.</p>} />
  </div>;
}
