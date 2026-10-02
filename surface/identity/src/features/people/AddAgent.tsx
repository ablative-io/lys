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
import { addAndRunCapability } from './registration-capability';
import type { AddAndRunCapability } from './registration-capability';
import { addAndRun, AddAndRunFailure, addAndRunStep, firstRunChoices, newAddAndRun, placementComputer, profileFromSettings, readAddAndRun } from './add-and-run';
import type { AddAndRun, FirstRunOptions } from './add-and-run';
import { ProfileFields } from '../provisioning/ProfileEditor';
import { validComputerName } from '../network/AddMachine';

function Form({ me, people, teams, search, capability, runOptions }: { me: MeView; people: PeopleView; teams: Team[]; search: string; capability: AddAndRunCapability; runOptions: FirstRunOptions }) {
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
  const [computerName, setComputerName] = useState(saved.walk?.placement.kind === 'new' ? saved.walk.placement.machine.body.name : '');
  const [computer, setComputer] = useState(saved.walk?.placement.kind === 'existing' ? saved.walk.placement.computer.id : runOptions.computers.length === 1 ? runOptions.computers[0].id : '');
  const selectedComputer = runOptions.computers.find((entry) => entry.id === computer);
  const inUse = useMemo(() => runOptions.network?.machines.filter((machine) => machine.state === 'in_use') ?? [], [runOptions.network]);
  const computerReady = inUse.length ? Boolean(selectedComputer) : validComputerName(computerName.trim());
  const hold = walk && runOptions.network === null
    ? new Refused(0, { refusal: 'RetainedNetworkUnconfirmed', reason: 'Lys could not confirm the computers for this saved request; it has not been sent.' })
    : walk?.placement.kind === 'new' && inUse.length > 0 && !inUse.some((machine) => machine.id === placementComputer(walk.placement).id)
      ? new Refused(0, { refusal: 'RetainedComputerAdditionHeld', reason: 'This saved request would add a second computer; it has not been sent.' })
      : walk?.placement.kind === 'existing' && (capability.machineAdmission !== true || !runOptions.computers.some((entry) => entry.id === placementComputer(walk.placement).id))
        ? new Refused(0, { refusal: 'RetainedComputerUnconfirmed', reason: 'The chosen computer, its local runner or its allowance route could not be confirmed. The saved request has not been sent; no replacement computer is chosen.' }) : null;
  const onePress = Boolean(walk) && !hold || !pending && !walk && runOptions.choices !== null;
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
    if (working.current || saved.error || hold || unavailable || !name.trim() || (!pending && !walk && (invalidChoice || taken))) return;
    if (onePress && !walk && (!settings || profileProblem || !computerReady)) return;
    working.current = true; setSending(true); setRefusal(null);
    try {
      let agent: string;
      if (onePress) {
        if (!walk && !settings) throw new Refused(0, { refusal: 'SettingsUnavailable', reason: 'Choose the settings before adding this agent.' });
        const initial = walk ?? newAddAndRun(newAgentRequest(name.trim(), person, team), settings ?? {}, computerName.trim(), me.person.id, selectedComputer);
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
      {onePress ? walk ? <p>Computer: {placementComputer(walk.placement).name}</p> : inUse.length ? runOptions.computers.length === 1 ? <p>Computer: {selectedComputer?.name}</p> : <label className="field">Computer<select name="computer" value={computer} onChange={(event) => setComputer(event.target.value)}><option value="">Choose a computer</option>{runOptions.computers.map((entry) => <option key={entry.id} value={entry.id}>{entry.name}</option>)}</select></label> : <label className="field">Computer name<input name="computer_name" value={computerName} required maxLength={100} autoComplete="off" onChange={(event) => setComputerName(event.target.value)} /><span className="hint">Type a name for this computer.</span></label> : null}
    </fieldset>
    {!walk && capability.machineAdmission === false ? inUse.map((machine) => <p key={machine.id}>{machine.may_run_roles?.length
      ? "Lys cannot confirm what " + machine.name + "'s roles grant, so it won't give one to " + (name.trim() || 'this agent') + '. Add the agent here, then start it from its page once it is admitted to ' + machine.name + '.'
      : machine.name + ' runs only the agents named when it was added. To add and run from here, add this computer again in Network with a role, then retire ' + machine.name + '.'}</p>) : null}
    {hold ? <><p className="why-not" role="alert">{hold.message}</p><details><summary>Details</summary><ErrorWords problem={hold} /></details></> : null}
    {inUse.length && (!onePress || hold) ? <p><a className="btn" href="#/network">Open Network</a></p> : null}
    {capability.problem ? <><p>The service’s registration choices could not be read. You can add an agent under You only.</p><details><summary>Registration details</summary><ErrorWords problem={capability.problem} /></details></> : !capability.answersTo ? <p>Adding an agent under another person is coming. You can add an agent under You.</p> : null}
    {unsupported ? <p role="alert" className="why-not">{unsupported}</p> : null}
    {invalidChoice ? <p role="alert" className="why-not">{invalidChoice}</p> : null}
    {runOptions.problem ? <><p>Lys could not read the choices for adding and running this agent.</p><details><summary>Details</summary><ErrorWords problem={runOptions.problem} /></details></> : null}
    {refusal ? refusal instanceof AddAndRunFailure ? <><p role="alert" className="why-not">{refusal.message}</p><details><summary>Details</summary><ErrorWords problem={refusal.problem} /></details></> : onePress ? <><p role="alert">Lys could not read the saved add-and-run request.</p><details><summary>Details</summary><ErrorWords problem={refusal} /></details></> : <ErrorWords problem={refusal} /> : null}
    {walk ? <p role="status">Lys has not finished {addAndRunStep(walk)}. {hold ? 'Its original requests are kept.' : 'Its original requests are saved; this button continues them.'}</p> : null}
    {onePress && !walk && profileProblem ? <><p>Choose the settings before adding and running this agent.</p><details><summary>Details</summary><p>{profileProblem}</p></details></> : null}
    {pending ? <p role="status">{pending.activated ? 'Your agent was added. Its team membership still needs confirmation.' : pending.agent ? 'Your agent was registered. Activation still needs confirmation.' : 'The registration has no confirmed answer yet.'} The original request is saved. Try that same request again without adding another agent.</p> : null}
    <p><button className="btn primary" type="submit" disabled={sending || Boolean(hold) || unavailable || !name.trim() || Boolean(saved.error) || (!pending && !walk && Boolean(taken || invalidChoice)) || (onePress && !walk && (Boolean(profileProblem) || !computerReady))}>{hold ? 'Saved request held' : onePress ? 'Add ' + (name.trim() || 'agent') + ' and run it on this computer' : sending ? 'Adding…' : pending ? 'Continue adding this agent' : 'Add agent'}</button></p>
  </form>;
  return onePress && runOptions.choices ? <ProfileFields profile={walk ? profileFromSettings(walk.settings) : null} choices={runOptions.choices} strict render={renderForm} /> : renderForm(null);
}

export function AddAgent() {
  const location = useLocation();
  const load = useLoad(async () => {
    const [me, people, teams, capability] = await Promise.all([api.me(), api.people(), readTeams(), addAndRunCapability()]);
    return { me, people, teams, capability, runOptions: await firstRunChoices(capability) };
  }, 'add-agent');
  return <div className="page"><a href="#/people">People and agents</a><h1>Add an agent</h1>
    <Gate load={load} title="Add an agent" ok={(data) => data.people.scope === 'directory' ? <Form key={location.search} me={data.me} people={data.people} teams={data.teams} search={location.search} capability={data.capability} runOptions={data.runOptions} /> : <p className="why-not">Only the Lys administrator can add agents. Ask them to add it for you.</p>} />
  </div>;
}
