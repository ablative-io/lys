import { ActionPicker, singleActionCarriers, withheldFromAgents } from '../grants/ActionPicker';
import type { ActionGroup } from '../grants/ActionPicker';
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
import { grantOptions, grantOptionKey, newAgentGrants, readGrantChoices } from './agent-grants';
import type { GrantChoices } from './agent-grants';
import { resourceNames, resourceWords } from './agent-resource-names';

function reportingChoices(people: PeopleView) {
  return people.people.flatMap((person) => [{ ...person, kind: 'person', owner: person.id }, ...person.agents.map((agent) => ({ ...agent, kind: 'agent', owner: person.id }))]);
}
function defaultName(people: PeopleView, target: string): string {
  const owner = reportingChoices(people).find((entry) => entry.id === target)?.owner;
  const names = new Set((people.people.find((entry) => entry.id === owner)?.agents ?? []).map((entry) => entry.display_name.trim().toLowerCase()));
  if (!names.has('new agent')) return 'New agent';
  for (let number = 2; number <= names.size + 2; number += 1) if (!names.has('new agent ' + number)) return 'New agent ' + number;
  throw new Error('AgentNameUnavailable: a fresh agent name could not be chosen.');
}

function Form({ me, people, teams, search, capability, runOptions, grants }: { me: MeView; people: PeopleView; teams: Team[]; search: string; capability: AddAndRunCapability; runOptions: FirstRunOptions; grants: GrantChoices }) {
  const navigate = useNavigate();
  const key = 'lys.add-agent.' + me.person.id;
  const runKey = 'lys.add-and-run.' + me.person.id;
  const [saved] = useState(() => {
    try {
      if (sessionStorage.getItem('lys.pending.register-agent') !== null) throw new Error('An earlier registration still needs its outcome checked before adding another agent. Ask the administrator to resolve that saved request.');
      const pending = readAgentRequest(key, me.person.id); const walk = readAddAndRun(runKey, me.person.id);
      if (pending && walk) throw new Refused(0, { refusal: 'EarlierChangeUnresolved', reason: 'An earlier registration and add-and-run request both need their outcomes checked.' });
      return { pending, walk, error: null };
    } catch (error) { return { pending: null, walk: null, error }; }
  });
  const query = new URLSearchParams(search);
  const restored = saved.walk?.registration ?? saved.pending;
  const firstTarget = restored ? restored.answersTo ?? me.person.id : query.get('answers_to') ?? me.person.id;
  const [name, setName] = useState(restored?.name ?? defaultName(people, firstTarget));
  const [automaticName, setAutomaticName] = useState(!restored);
  const [person, setPerson] = useState(restored ? restored.answersTo ?? me.person.id : query.get('answers_to') ?? me.person.id);
  const [team, setTeam] = useState(restored ? restored.team ?? '' : query.get('team') ?? '');
  const [sending, setSending] = useState(false);
  const [refusal, setRefusal] = useState<unknown>(saved.error);
  const [pending, setPending] = useState(saved.pending);
  const [walk, setWalk] = useState(saved.walk);
  const [selectedGrants, setSelectedGrants] = useState(restored?.grants.map((entry) => entry.source + ':' + entry.relation) ?? []);
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
  const targets = useMemo(() => reportingChoices(people), [people]);
  const chosen = targets.find((entry) => entry.id === person);
  const names = useMemo(() => new Map((people.people.find((entry) => entry.id === chosen?.owner)?.agents ?? []).map((agent) => [agent.display_name.trim().toLowerCase(), agent.display_name])), [people, chosen?.owner]);
  const activeTeams = useMemo(() => teams.filter((entry) => entry.state === 'active'), [teams]);
  const options = useMemo(() => grantOptions(grants, person, me.person.id, chosen?.display_name ?? 'the holder'), [grants, person, me.person.id, chosen?.display_name]);
  const resourceLabels = useMemo(() => resourceNames(people, teams, runOptions.network), [people, teams, runOptions.network]);
  const actionGroups = useMemo(() => {
    const groups = new Map<string, ActionGroup>();
    for (const option of options) {
      const resource = option.grant.resource;
      const id = JSON.stringify(resource);
      const label = resourceWords(resource, resourceLabels);
      const group = groups.get(id) ?? { id, title: label.words, href: label.unnamed ? '#/resources' : undefined, choices: [] };
      group.choices.push({ id: grantOptionKey(option), resource, relation: option.relation ?? '', actions: option.actions, reason: option.reason });
      groups.set(id, group);
    }
    return [...groups.values()];
  }, [options, resourceLabels]);
  const unpickable = useMemo(() => {
    const model = grants.model;
    const withheld = withheldFromAgents(model);
    if (!model || withheld === null) return false;
    const relations = singleActionCarriers(model);
    return grants.grants.some((grant) => grant.holder === person && !grant.resource.kind.includes('.')
      && grant.actions.some((action) => !withheld.includes(action) && !relations.has(action)));
  }, [grants, person]);
  const unavailable = !capability.answersTo && person !== me.person.id;
  const unsupported = unavailable ? pending || walk ? 'Your saved request names another person or agent. This service cannot accept that choice; the saved request has not been sent.' : 'This service cannot register an agent under that person or agent.' : '';
  const taken = pending || walk ? undefined : names.get(name.trim().toLowerCase());
  const invalidChoice = !chosen || chosen.state !== 'active' ? 'Choose an active person or agent for this agent to answer to.'
    : team && !activeTeams.some((entry) => entry.id === team) ? 'The chosen team is unavailable. Choose an active team or no team.' : '';
  const duplicate = taken ? chosen?.display_name + ' already has an agent named ' + taken + '. Choose another name.' : '';
  const retain = (where: string, next: PendingAgent | AddAndRun) => {
    try { sessionStorage.setItem(where, JSON.stringify(next)); }
    catch { throw new Refused(0, { refusal: 'PendingAgentRetentionFailed', reason: 'This browser could not save the next request. It has not been sent; check the outcomes of earlier saved requests before continuing.' }); }
  };
  const keep = (next: PendingAgent) => { retain(key, next); setPending(next); };
  const keepWalk = (next: AddAndRun) => { retain(runKey, next); setWalk(next); };
  const submit = async (event: FormEvent<HTMLFormElement>, settings?: Record<string, unknown>, profileProblem = '') => {
    event.preventDefault();
    if (working.current || saved.error || hold || unavailable || !name.trim() || (!pending && !walk && (invalidChoice || taken))) return;
    if (onePress && !walk && (!settings || profileProblem || !computerReady)) return;
    working.current = true; setSending(true); setRefusal(null);
    try {
      let agent: string;
      const registration = pending ?? walk?.registration ?? newAgentRequest(name.trim(), person, team, newAgentGrants(options, selectedGrants));
      if (onePress) {
        if (!walk && !settings) throw new Refused(0, { refusal: 'SettingsUnavailable', reason: 'Choose the settings before adding this agent.' });
        const initial = walk ?? newAddAndRun(registration, settings ?? {}, computerName.trim(), me.person.id, selectedComputer);
        agent = await addAndRun(initial, me.signed_in, keepWalk);
        sessionStorage.removeItem(runKey);
      } else {
        agent = await addAgent(registration, me.person.id, keep, me.signed_in);
        sessionStorage.removeItem(key);
      }
      navigate('/team/' + encodeURIComponent(agent), { replace: true });
    } catch (problem) {
      // The agent is added and only its folder is missing: a definite answer, so the
      // saved request is over and the team screen asks for the folder where Start is.
      let added: string | null = null;
      if (problem instanceof AddAndRunFailure && problem.problem instanceof Refused && problem.problem.refusal.refusal === 'WorkingFolderUnnamed') {
        // Deliberate: when the saved request cannot be read back, nothing is assumed about
        // which agent was added, and the refusal itself is shown below instead.
        try { added = readAddAndRun(runKey, me.person.id)?.registration.agent ?? null; } catch { added = null; }
      }
      if (added) { sessionStorage.removeItem(runKey); navigate('/team/' + encodeURIComponent(added), { replace: true }); return; }
      setRefusal(problem);
    }
    finally { working.current = false; setSending(false); }
  };
  const locked = sending || pending !== null || walk !== null || Boolean(saved.error);
  const registrationFields = <>
    <label className="field">Name
      <input name="display_name" value={name} autoFocus autoComplete="off" disabled={locked} onChange={(event) => { setName(event.target.value); setAutomaticName(false); setRefusal(null); }} aria-invalid={taken ? true : undefined} aria-describedby={taken ? 'add-agent-duplicate' : undefined} />
    </label>
    {duplicate ? <p className="why-not" id="add-agent-duplicate" role="alert">{duplicate}</p> : null}
    <label className="field">Who this agent answers to<select name="answers_to" value={person} disabled={locked} onChange={(event) => { setPerson(event.target.value); setSelectedGrants([]); if (automaticName) setName(defaultName(people, event.target.value)); }}>
      {!chosen ? <option value={person}>Reporting target unavailable</option> : null}
      {targets.map((entry) => <option key={entry.id} value={entry.id} disabled={entry.state !== 'active' || !capability.answersTo && entry.id !== me.person.id}>{entry.id === me.person.id ? 'You (' + entry.display_name + ')' : entry.display_name}{entry.kind === 'agent' ? ' · agent' : ''}{entry.state !== 'active' ? ' · ' + entry.state : !capability.answersTo && entry.id !== me.person.id ? ' · not served' : ''}</option>)}
    </select></label>
    <fieldset style={{ border: 0, padding: 0, margin: '16px 0' }}><legend>What this agent may do</legend>
      {chosen?.kind === 'agent' ? <p>This form cannot pass on {chosen.display_name}’s access. Add {name.trim() || 'this agent'} without extra access; <a href={'#/file/' + encodeURIComponent(chosen.id) + '/access'}>review {chosen.display_name}’s access</a>.</p> : <>
        {grants.model && withheldFromAgents(grants.model) !== null ? <ActionPicker model={grants.model} groups={actionGroups} selected={selectedGrants} change={setSelectedGrants} disabled={locked} />
          : !grants.problem ? <p>Agent access choices are unavailable until Lys declares which actions are withheld from agents. Nothing is selected.</p> : null}
        {unpickable ? <p>Some actions cannot be selected because the model has no relation carrying that action alone.</p> : null}
        {!options.length && !grants.problem ? <p>No grants were returned for {chosen?.display_name ?? 'this reporting target'}.</p> : null}
        {grants.problem ? <ErrorWords problem={grants.problem} /> : null}
      </>}
    </fieldset>
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
    {hold ? <ErrorWords problem={hold} /> : null}
    {inUse.length && (!onePress || hold) ? <p><a className="btn" href="#/network">Open Network</a></p> : null}
    {capability.problem ? <><p>The service’s registration choices could not be read. You can add an agent under You only.</p><ErrorWords problem={capability.problem} /></> : !capability.answersTo ? <p>This service can register an agent under You only.</p> : null}
    {unsupported ? <p role="alert" className="why-not">{unsupported}</p> : null}
    {invalidChoice ? <p role="alert" className="why-not">{invalidChoice}</p> : null}
    {runOptions.problem ? <><p>Lys could not read the choices for adding and running this agent.</p><ErrorWords problem={runOptions.problem} /></> : null}
    {refusal ? refusal instanceof AddAndRunFailure ? <><p role="alert" className="why-not">{refusal.message}</p><ErrorWords problem={refusal.problem} /></> : <ErrorWords problem={refusal} /> : null}
    {walk ? <p role="status">Lys has not finished {addAndRunStep(walk)}. {hold ? 'Its original requests are kept.' : 'Its original requests are saved; this button continues them.'}</p> : null}
    {onePress && !walk && profileProblem ? <p className="why-not" role="alert">{profileProblem}</p> : null}
    {pending ? <p role="status">{pending.activated ? 'Your agent was added. Its team membership still needs confirmation.' : pending.agent ? 'Your agent was registered. Activation still needs confirmation.' : 'The registration has no confirmed answer yet.'} The original request is saved. Try that same request again without adding another agent.</p> : null}
    {/* The button is never greyed without its reason beside it: the reasons not already said above are said here. */}
    {!sending && !hold && !unavailable && !saved.error && !name.trim() ? <p className="why-not" role="alert">Give this agent a name.</p>
      : !sending && !hold && !unavailable && !saved.error && onePress && !walk && !profileProblem && !computerReady ? <p className="why-not" role="alert">{inUse.length ? 'Choose the computer this agent runs on.' : 'Give this computer a name.'}</p> : null}
    <p><button className="btn primary" type="submit" disabled={sending || Boolean(hold) || unavailable || !name.trim() || Boolean(saved.error) || (!pending && !walk && Boolean(taken || invalidChoice)) || (onePress && !walk && (Boolean(profileProblem) || !computerReady))}>{hold ? 'Saved request held' : onePress ? 'Add ' + (name.trim() || 'agent') + ' and run it on this computer' : sending ? 'Adding…' : pending ? 'Continue adding this agent' : 'Add agent'}</button></p>
  </form>;
  return onePress && runOptions.choices ? <ProfileFields profile={walk ? profileFromSettings(walk.settings) : null} choices={runOptions.choices} firstRun computer={walk ? placementComputer(walk.placement).id : selectedComputer?.id ?? ''} render={renderForm} /> : renderForm(null);
}

export function AddAgent() {
  const location = useLocation();
  const load = useLoad(async () => {
    const [me, people, teams, capability, grants] = await Promise.all([api.me(), api.people(), readTeams(), addAndRunCapability(), readGrantChoices()]);
    return { me, people, teams, capability, grants, runOptions: await firstRunChoices(capability) };
  }, 'add-agent');
  return <div className="page"><a href="#/people">People and agents</a><h1>Add an agent</h1>
    <Gate load={load} title="Add an agent" ok={(data) => data.people.scope === 'directory' ? <Form key={location.search} me={data.me} people={data.people} teams={data.teams} search={location.search} capability={data.capability} runOptions={data.runOptions} grants={data.grants} /> : <p className="why-not">Only the Lys administrator can add agents. Ask them to add it for you.</p>} />
  </div>;
}
