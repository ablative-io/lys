import { useMemo, useRef, useState } from 'react';
import type { FormEvent, ReactNode } from 'react';
import { useLocation, useNavigate } from 'react-router';
import { api, Refused, useLoad } from '../../api';
import type { MeView, PeopleView } from '../../generated';
import { DirectoryGate as Gate, ErrorWords } from './Words';
import { readTeams } from '../teams/Teams';
import type { Team } from '../teams/contract';
import { addAgent, newAgentRequest, readAgentRequest } from './add-agent-request';
import type { PendingAgent } from './add-agent-request';
import { registrationCapability } from './registration-capability';
import type { RegistrationCapability } from './registration-capability';
import { addAndRun, AddAndRunFailure, addAndRunStep, firstRunChoices, newAddAndRun, profileFromSettings, readAddAndRun } from './add-and-run';
import type { AddAndRun } from './add-and-run';
import { ProfileFields } from '../provisioning/ProfileEditor';
import type { Choices } from '../provisioning/choices';
import { validComputerName } from '../network/AddMachine';

function Form({ me, people, teams, search, capability, runOptions }: { me: MeView; people: PeopleView; teams: Team[]; search: string; capability: RegistrationCapability; runOptions: { choices: Choices | null; problem: unknown } }) {
  const navigate = useNavigate();
  const key = 'lys.add-agent.' + me.person.id;
  const runKey = 'lys.add-and-run.' + me.person.id;
  const [saved] = useState(() => {
    try {
      if (sessionStorage.getItem('lys.pending.register-agent') !== null) throw new Error('An earlier registration still needs its outcome checked before adding another agent. Ask the administrator to resolve that saved request.');
      const pending = readAgentRequest(key); const walk = readAddAndRun(runKey, me.person.id);
      if (pending && walk) throw new Refused(0, { refusal: 'EarlierChangeUnresolved', reason: 'An earlier registration and add-and-run request both need their outcomes checked.' });
      return { pending, walk, error: null };
    } catch (error) { return { pending: null, walk: null, error }; }
  });
  const query = new URLSearchParams(search);
  const restored = saved.walk?.registration ?? saved.pending;
  const [name, setName] = useState(restored?.name ?? '');
  const [person, setPerson] = useState(restored ? restored.answersTo ?? me.person.id : query.get('answers_to') ?? me.person.id);
  const [team, setTeam] = useState(restored ? restored.team ?? '' : query.get('team') ?? '');
  const [sending, setSending] = useState(false);
  const [refusal, setRefusal] = useState<unknown>(saved.error);
  const [pending, setPending] = useState(saved.pending);
  const [walk, setWalk] = useState(saved.walk);
  const [computerName, setComputerName] = useState(saved.walk?.machine.body.name ?? '');
  const onePress = Boolean(walk) || !pending && runOptions.choices !== null;
  const working = useRef(false);
  const peopleById = useMemo(() => new Map(people.people.map((entry) => [entry.id, entry])), [people]);
  const names = useMemo(() => new Map((peopleById.get(person)?.agents ?? []).map((agent) => [agent.display_name.trim().toLowerCase(), agent.display_name])), [peopleById, person]);
  const activeTeams = useMemo(() => teams.filter((entry) => entry.state === 'active'), [teams]);
  const chosen = peopleById.get(person);
  const unavailable = !capability.answersTo && person !== me.person.id;
  const unsupported = unavailable ? pending ? 'Your saved request names another person. This service cannot accept that choice yet; the saved request has not been sent.' : 'Adding an agent under another person is coming. Choose You to add an agent now.' : '';
  const taken = pending || walk ? undefined : names.get(name.trim().toLowerCase());
  const invalidChoice = !chosen || chosen.state !== 'active' ? 'Choose an active person for this agent to answer to.'
    : team && !activeTeams.some((entry) => entry.id === team) ? 'The chosen team is unavailable. Choose an active team or no team.' : '';
  const duplicate = taken ? chosen?.display_name + ' already has an agent named ' + taken + '. Choose another name.' : '';
  const keep = (next: PendingAgent) => { sessionStorage.setItem(key, JSON.stringify(next)); setPending(next); };
  const keepWalk = (next: AddAndRun) => { sessionStorage.setItem(runKey, JSON.stringify(next)); setWalk(next); };
  const submit = async (event: FormEvent<HTMLFormElement>, settings?: Record<string, unknown>, profileProblem = '') => {
    event.preventDefault();
    if (working.current || saved.error || unavailable || !name.trim() || (!pending && !walk && (invalidChoice || taken))) return;
    if (onePress && !walk && (!settings || profileProblem || !validComputerName(computerName.trim()))) return;
    working.current = true; setSending(true); setRefusal(null);
    try {
      let agent: string;
      if (onePress) {
        if (!walk && !settings) throw new Refused(0, { refusal: 'SettingsUnavailable', reason: 'Choose the settings before adding this agent.' });
        const initial = walk ?? newAddAndRun(newAgentRequest(name.trim(), person, team), settings ?? {}, computerName.trim(), me.person.id);
        agent = await addAndRun(initial, me.signed_in, keepWalk);
        sessionStorage.removeItem(runKey);
      } else {
        agent = await addAgent(pending ?? newAgentRequest(name.trim(), person, team), me.person.id, keep, me.signed_in);
        sessionStorage.removeItem(key);
      }
      navigate('/file/' + encodeURIComponent(agent), { replace: true });
    } catch (problem) { setRefusal(problem); }
    finally { working.current = false; setSending(false); }
  };
  const locked = sending || pending !== null || walk !== null || Boolean(saved.error);
  const registrationFields = <>
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
  </>;
  const renderForm = (fields: ReactNode, settings?: Record<string, unknown>, profileProblem = '') => <form className="add-agent" aria-label="Add an agent" onSubmit={(event) => { void submit(event, settings, profileProblem); }}>
    <fieldset disabled={locked} style={{ border: 0, padding: 0, margin: 0 }}>
      {registrationFields}{fields}
      {onePress ? <label className="field">Computer name<input name="computer_name" value={computerName} required maxLength={100} autoComplete="off" onChange={(event) => setComputerName(event.target.value)} /><span className="hint">Type a name for this computer.</span></label> : null}
    </fieldset>
    {capability.problem ? <><p>The service’s registration choices could not be read. You can add an agent under You only.</p><details><summary>Registration details</summary><ErrorWords problem={capability.problem} /></details></> : !capability.answersTo ? <p>Adding an agent under another person is coming. You can add an agent under You.</p> : null}
    {unsupported ? <p role="alert" className="why-not">{unsupported}</p> : null}
    {invalidChoice ? <p role="alert" className="why-not">{invalidChoice}</p> : null}
    {runOptions.problem ? <><p>Lys could not read the choices for adding and running this agent.</p><details><summary>Details</summary><ErrorWords problem={runOptions.problem} /></details></> : null}
    {refusal ? refusal instanceof AddAndRunFailure ? <><p role="alert" className="why-not">{refusal.message}</p><details><summary>Details</summary><ErrorWords problem={refusal.problem} /></details></> : onePress ? <><p role="alert">Lys could not read the saved add-and-run request.</p><details><summary>Details</summary><ErrorWords problem={refusal} /></details></> : <ErrorWords problem={refusal} /> : null}
    {walk ? <p role="status">Lys has not finished {addAndRunStep(walk)}. Its original requests are saved; this button continues them.</p> : null}
    {onePress && !walk && profileProblem ? <><p>Choose the settings before adding and running this agent.</p><details><summary>Details</summary><p>{profileProblem}</p></details></> : null}
    {pending ? <p role="status">{pending.activated ? 'Your agent was added. Its team membership still needs confirmation.' : pending.agent ? 'Your agent was registered. Activation still needs confirmation.' : 'The registration has no confirmed answer yet.'} The original request is saved. Try that same request again without adding another agent.</p> : null}
    <p><button className="btn primary" type="submit" disabled={sending || unavailable || !name.trim() || Boolean(saved.error) || (!pending && !walk && Boolean(taken || invalidChoice)) || (onePress && !walk && (Boolean(profileProblem) || !validComputerName(computerName.trim())))}>{onePress ? 'Add ' + (name.trim() || 'agent') + ' and run it on this computer' : sending ? 'Adding…' : pending ? 'Continue adding this agent' : 'Add agent'}</button></p>
  </form>;
  return onePress && runOptions.choices ? <ProfileFields profile={walk ? profileFromSettings(walk.settings) : null} choices={runOptions.choices} strict render={renderForm} /> : renderForm(null);
}

export function AddAgent() {
  const location = useLocation();
  const load = useLoad(async () => {
    const [me, people, teams, capability] = await Promise.all([api.me(), api.people(), readTeams(), registrationCapability()]);
    return { me, people, teams, capability, runOptions: await firstRunChoices() };
  }, 'add-agent');
  return <div className="page"><a href="#/people">People and agents</a><h1>Add an agent</h1>
    <Gate load={load} title="Add an agent" ok={(data) => data.people.scope === 'directory' ? <Form key={location.search} me={data.me} people={data.people} teams={data.teams} search={location.search} capability={data.capability} runOptions={data.runOptions} /> : <p className="why-not">Only the Lys administrator can add agents. Ask them to add it for you.</p>} />
  </div>;
}
