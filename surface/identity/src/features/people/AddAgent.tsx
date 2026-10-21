import { singleActionCarriers, withheldFromAgents } from '../grants/ActionPicker';
import { AgentGrantPicker } from './AgentGrantPicker';
import type { GrantGroup } from './AgentGrantPicker';
import { useEffect, useMemo, useRef, useState } from 'react';
import type { FormEvent, ReactNode } from 'react';
import { useLocation, useNavigate } from 'react-router';
import { api, Refused, useLoad } from '../../api';
import type { MeView, PeopleView } from '../../generated';
import { DirectoryGate as Gate, ErrorWords, STATUS } from './Words';
import { ClearSavedRequest } from './SavedRequest';
import { readTeams } from '../teams/Teams';
import type { Team } from '../teams/contract';
import { addAgent, newAgentRequest, readAgentRequest } from './add-agent-request';
import type { PendingAgent } from './add-agent-request';
import { addAndRunCapability } from './registration-capability';
import type { AddAndRunCapability } from './registration-capability';
import { addAndRun, AddAndRunFailure, addAndRunStep, CONNECT_COMPUTER, firstRunChoices, newAddAndRun, placementComputer, profileFromSettings, readAddAndRun, runnerProblemReason } from './add-and-run';
import type { AddAndRun, FirstRunOptions } from './add-and-run';
import { ProfileFields } from '../provisioning/ProfileEditor';
import { validComputerName } from '../network/AddMachine';
import { grantOptions, grantOptionKey, newAgentGrants, readGrantChoices } from './agent-grants';
import type { GrantChoices } from './agent-grants';
import { resourceNames, resourceWords } from './agent-resource-names';
import { keepRecord, releaseRecord } from '../../kept';
import { Act } from '../../shell/Act';
import { PageHeader } from '../../shell/PageHeader';
import './add-agent.css';
import { AppsFirst, anyAppResource } from '../grants/AppsFirst';

function reportingChoices(people: PeopleView) {
  return people.people.flatMap((person) => [{ ...person, kind: 'person', owner: person.id }, ...person.agents.map((agent) => ({ ...agent, kind: 'agent', owner: person.id }))]);
}

/** The legacy key an earlier Lys kept an unanswered registration under; nothing writes it now, and it cannot be sent again. */
const LEGACY = 'lys.pending.register-agent';

/**
 * The saved requests this tab holds, read once per form. `later` keeps a saved add-and-run back while a saved plain
 * registration is sent again first: both stay kept, and the add-and-run is read again once the registration is settled.
 */
function savedRequests(key: string, runKey: string, person: string, later: boolean) {
  try {
    if (sessionStorage.getItem(LEGACY) !== null) throw new Error('SavedRequestUnreadable: an earlier registration was kept in a form this Lys cannot send again.');
    const pending = readAgentRequest(key, person); const walk = later && pending ? null : readAddAndRun(runKey, person);
    if (pending && walk) throw new Refused(0, { refusal: 'EarlierChangeUnresolved', reason: 'An earlier registration and an add-and-run request are both saved without a confirmed answer.' });
    return { pending, walk, error: null, both: false };
  } catch (error) { return { pending: null, walk: null, error, both: error instanceof Refused && error.refusal.refusal === 'EarlierChangeUnresolved' }; }
}

/** The saved records that cannot be read back: only these are released by clearing, never a readable saved request. */
function unreadableRecords(key: string, runKey: string, person: string): string[] {
  const reads: [string, () => unknown][] = [[key, () => readAgentRequest(key, person)], [runKey, () => readAddAndRun(runKey, person)]];
  return [LEGACY, ...reads.filter(([, read]) => { try { read(); return false; } catch { return true; } }).map(([where]) => where)];
}

function Form({ me, people, teams, search, capability, runOptions, grants, later, restart }: { me: MeView; people: PeopleView; teams: Team[]; search: string; capability: AddAndRunCapability; runOptions: FirstRunOptions; grants: GrantChoices; later: boolean; restart: (later: boolean) => void }) {
  const navigate = useNavigate();
  const key = 'lys.add-agent.' + me.person.id;
  const runKey = 'lys.add-and-run.' + me.person.id;
  const [saved] = useState(() => savedRequests(key, runKey, me.person.id, later));
  const query = new URLSearchParams(search);
  const restored = saved.walk?.registration ?? saved.pending;
  // The name starts empty: an agent is never added under a name nobody typed, and Add stays off until one is.
  const [name, setName] = useState(restored?.name ?? '');
  const [person, setPerson] = useState(restored ? restored.answersTo ?? me.person.id : query.get('answers_to') ?? me.person.id);
  const [team, setTeam] = useState(restored ? restored.team ?? '' : query.get('team') ?? '');
  const [sending, setSending] = useState(false);
  const [refusal, setRefusal] = useState<unknown>(saved.error);
  const [pending, setPending] = useState(saved.pending);
  const [walk, setWalk] = useState(saved.walk);
  const [selectedGrants, setSelectedGrants] = useState(restored?.grants.map((entry) => entry.source + ':' + entry.relation + (entry.resource.kind.includes('.') ? ':' + entry.actions[0] : '')) ?? []);
  // A new computer is this one: its name starts as the name the computer gives itself, and stays editable.
  const [computerName, setComputerName] = useState(saved.walk?.placement.kind === 'new' ? saved.walk.placement.machine.body.name : runOptions.network?.host_name ?? '');
  const [namedComputer, setNamedComputer] = useState(Boolean(saved.walk));
  const hostName = runOptions.network?.host_name ?? '';
  useEffect(() => {
    if (!namedComputer && hostName && !computerName) setComputerName(hostName);
  }, [namedComputer, hostName, computerName]);
  // Lys cannot tell which computer this browser is on, so no runner is chosen for being first to answer: a computer is
  // chosen for the person only when every runner has answered and exactly one is confirmed; otherwise the choice is theirs.
  const asking = runOptions.runners.some((entry) => entry.state === 'asking');
  const sole = !asking && runOptions.computers.length === 1 ? runOptions.computers[0] : undefined;
  const soleId = sole?.id ?? '';
  const [computer, setComputer] = useState(saved.walk?.placement.kind === 'existing' ? saved.walk.placement.computer.id : soleId);
  useEffect(() => {
    if (!computer && !walk && soleId) setComputer(soleId);
  }, [computer, walk, soleId]);
  const selectedComputer = runOptions.computers.find((entry) => entry.id === computer);
  const administrator = people.scope === 'directory';
  const needsComputer = !administrator && !runOptions.loading && !runOptions.computers.length;
  const runComputerName = walk ? placementComputer(walk.placement).name : selectedComputer?.name ?? computerName.trim();
  const inUse = useMemo(() => runOptions.network?.machines.filter((machine) => machine.state === 'in_use') ?? [], [runOptions.network]);
  const computerReady = inUse.length ? Boolean(selectedComputer) : validComputerName(computerName.trim());
  const hold = walk?.placement.kind === 'new' && !administrator
    ? new Refused(0, { refusal: 'ComputerConnectionRequired', reason: CONNECT_COMPUTER })
    : walk && runOptions.network === null
    ? new Refused(0, { refusal: 'RetainedNetworkUnconfirmed', reason: 'Lys could not confirm the computers for this saved request; it has not been sent.' })
    : walk?.placement.kind === 'new' && inUse.length > 0 && !inUse.some((machine) => machine.id === placementComputer(walk.placement).id)
      ? new Refused(0, { refusal: 'RetainedComputerAdditionHeld', reason: 'This saved request would add a second computer; it has not been sent.' })
      : walk?.placement.kind === 'existing' && (capability.machineAdmission !== true || !runOptions.computers.some((entry) => entry.id === placementComputer(walk.placement).id))
        ? new Refused(0, { refusal: 'RetainedComputerUnconfirmed', reason: 'The chosen computer, its local runner or its allowance route could not be confirmed. The saved request has not been sent; no replacement computer is chosen.' }) : null;
  const onePress = Boolean(walk) && !hold || !pending && !walk && runOptions.choices !== null;
  // A runner's problem stops only the run path: adding now and starting later never waits on a runner. While the
  // choices are still being read the button is not yet known to be one or the other, except for a saved add-only request.
  const runUnavailable = runOptions.loading && !pending || onePress && Boolean(runOptions.problem);
  const working = useRef(false);
  const [showRetired, setShowRetired] = useState(false);
  const targets = useMemo(() => reportingChoices(people).filter((entry) => people.scope === 'directory' || entry.id === me.person.id)
    .sort((left, right) => Number(right.id === me.person.id) - Number(left.id === me.person.id)), [people, me.person.id]);
  const retiredCount = targets.filter((entry) => entry.state === 'retired' || entry.state === 'suspended').length;
  const shownTargets = targets.filter((entry) => showRetired || entry.state !== 'retired' && entry.state !== 'suspended');
  const chosen = targets.find((entry) => entry.id === person);
  const names = useMemo(() => new Map((people.people.find((entry) => entry.id === chosen?.owner)?.agents ?? []).map((agent) => [agent.display_name.trim().toLowerCase(), agent.display_name])), [people, chosen?.owner]);
  const activeTeams = useMemo(() => teams.filter((entry) => entry.state === 'active'), [teams]);
  const options = useMemo(() => grantOptions(grants, person, me.person.id, chosen?.display_name ?? 'the holder'), [grants, person, me.person.id, chosen?.display_name]);
  const resourceLabels = useMemo(() => resourceNames(people, teams, runOptions.network), [people, teams, runOptions.network]);
  const actionGroups = useMemo(() => {
    const groups = new Map<string, GrantGroup>();
    for (const option of options) {
      const resource = option.grant.resource;
      const id = JSON.stringify(resource);
      const label = resourceWords(resource, resourceLabels);
      const group = groups.get(id) ?? { id, title: label.words, href: label.unnamed ? '#/resources' : undefined, choices: [] };
      group.choices.push({ id: grantOptionKey(option), resource, actions: option.actions, reason: option.reason });
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
    try { keepRecord(where, next); }
    catch { throw new Refused(0, { refusal: 'PendingAgentRetentionFailed', reason: 'This browser could not save the next request. It has not been sent; check the outcomes of earlier saved requests before continuing.' }); }
  };
  const keep = (next: PendingAgent) => { retain(key, next); setPending(next); };
  const keepWalk = (next: AddAndRun) => { retain(runKey, next); setWalk(next); };
  const submit = async (event: FormEvent<HTMLFormElement>, settings?: Record<string, unknown>, profileProblem = '') => {
    event.preventDefault();
    if (working.current || saved.error || hold || runUnavailable || unavailable || !name.trim() || (!pending && !walk && (invalidChoice || taken))) return;
    if (onePress && !walk && (!settings || profileProblem || !computerReady)) return;
    working.current = true; setSending(true); setRefusal(null);
    try {
      let agent: string;
      const registration = pending ?? walk?.registration ?? newAgentRequest(name.trim(), person, team, newAgentGrants(options, selectedGrants));
      if (onePress) {
        if (!walk && !settings) throw new Refused(0, { refusal: 'SettingsUnavailable', reason: 'Choose the settings before adding this agent.' });
        const initial = walk ?? newAddAndRun(registration, settings ?? {}, computerName.trim(), me.person.id, selectedComputer, administrator);
        agent = await addAndRun(initial, me.signed_in, keepWalk, administrator);
        releaseRecord(runKey);
      } else {
        agent = await addAgent(registration, me.person.id, keep, me.signed_in);
        releaseRecord(key);
      }
      navigate('/file/' + encodeURIComponent(agent), { replace: true });
    } catch (problem) {
      // The agent is added and only its folder is missing: a definite answer, so the
      // saved request is over and the team screen asks for the folder where Start is.
      let added: string | null = null;
      if (problem instanceof AddAndRunFailure && problem.problem instanceof Refused && problem.problem.refusal.refusal === 'WorkingFolderUnnamed') {
        // Deliberate: when the saved request cannot be read back, nothing is assumed about
        // which agent was added, and the refusal itself is shown below instead.
        try { added = readAddAndRun(runKey, me.person.id)?.registration.agent ?? null; } catch { added = null; }
      }
      if (added) { releaseRecord(runKey); navigate('/file/' + encodeURIComponent(added), { replace: true }); return; }
      setRefusal(problem);
    }
    finally { working.current = false; setSending(false); }
  };
  const locked = sending || pending !== null || walk !== null || Boolean(saved.error);
  const registrationFields = <>
    <label className="field">Name
      <input name="display_name" value={name} autoFocus autoComplete="off" disabled={locked} onChange={(event) => { setName(event.target.value); setRefusal(null); }} aria-invalid={taken ? true : undefined} aria-describedby={taken ? 'add-agent-duplicate' : undefined} />
    </label>
    {duplicate ? <p className="why-not" id="add-agent-duplicate" role="alert">{duplicate}</p> : null}
  </>;
  // Who it answers to, its access and its team: asked on a first run only under More settings, with their defaults kept.
  const moreRegistration = <>
    <label className="field">Who this agent answers to<select name="answers_to" value={person} disabled={locked} onChange={(event) => { setPerson(event.target.value); setSelectedGrants([]); }}>
      {shownTargets.filter((entry) => capability.answersTo || entry.id === me.person.id || entry.id === person).map((entry) => <option key={entry.id} value={entry.id} disabled={entry.state !== 'active' || !capability.answersTo && entry.id !== me.person.id}>{entry.id === me.person.id ? 'You (' + entry.display_name + ')' : entry.display_name}{entry.kind === 'agent' ? ' · agent' : ''}{entry.state !== 'active' ? ' · ' + (STATUS[entry.state]?.toLowerCase() ?? entry.state) : ''}</option>)}
      {!shownTargets.some((entry) => entry.id === person) ? <option value={person} disabled>{chosen ? chosen.display_name + ' · ' + chosen.state : 'Reporting target unavailable'}</option> : null}
    </select></label>
    {retiredCount && capability.answersTo ? <Act symbol="retire" name={showRetired ? 'Hide retired and paused' : 'Show retired and paused (' + retiredCount + ')'} word={showRetired ? 'Hide retired and paused' : 'Show retired and paused (' + retiredCount + ')'} aria-pressed={showRetired} disabled={locked} onClick={() => setShowRetired(!showRetired)} /> : null}
    <fieldset className="agent-grants-scope"><legend>What this agent is given access to in Lys</legend>
      {chosen?.kind === 'agent' ? <p>This form cannot pass on {chosen.display_name}’s access. Add {name.trim() || 'this agent'} without extra access; <a href={'#/file/' + encodeURIComponent(chosen.id) + '/access'}>review {chosen.display_name}’s access</a>.</p> : <>
        {grants.model && withheldFromAgents(grants.model) !== null ? <AgentGrantPicker model={grants.model} groups={actionGroups} selected={selectedGrants} change={setSelectedGrants} disabled={locked} />
          : !grants.problem ? <p>Lys cannot offer access for agents here yet, so this agent is added with none. You can give it access afterwards, on its own page.</p> : null}
        {unpickable ? <p>Some kinds of access cannot be given one at a time, so they are not offered here.</p> : null}
        {!options.length && !grants.problem ? <p>{chosen?.display_name ?? 'The one this agent answers to'} has no access in Lys to pass on.</p> : null}
        {!grants.problem && !anyAppResource(options.map((option) => option.grant.resource)) ? <AppsFirst /> : null}
        {grants.problem ? <ErrorWords problem={grants.problem} /> : null}
      </>}
    </fieldset>
    <label className="field">Team this agent joins<select name="team" value={team} disabled={locked} onChange={(event) => setTeam(event.target.value)}>
      <option value="">No team</option>
      {team && !activeTeams.some((entry) => entry.id === team) ? <option value={team}>Team unavailable</option> : null}
      {activeTeams.map((entry) => <option key={entry.id} value={entry.id}>{entry.name}</option>)}
    </select></label>
  </>;
  const firstRunForm = onePress && Boolean(runOptions.choices);
  // The button says "and run" only when a start can follow: without a working folder the agent is added and allowed,
  // and its page asks for the folder before it starts, so the press never promises a run it cannot make.
  const runsWith = (settings?: Record<string, unknown>) => {
    const folder = walk ? walk.settings.working_folder : settings?.working_folder;
    return typeof folder === 'string' && folder.trim() !== '';
  };
  const renderForm = (fields: ReactNode, settings?: Record<string, unknown>, profileProblem = '') => <form className="add-agent" aria-label="Add an agent" onSubmit={(event) => { void submit(event, settings, profileProblem); }}>
    <fieldset disabled={locked} style={{ border: 0, padding: 0, margin: 0 }}>
      <div className={fields ? 'add-agent-two' : 'add-agent-one'}>
        <div>{registrationFields}{firstRunForm ? null : moreRegistration}
        {!needsComputer && (onePress || runOptions.runners.length) ? walk ? <p>Computer: {placementComputer(walk.placement).name}</p> : inUse.length ? <label className="field">Computer this agent runs on<select name="computer" value={computer} onChange={(event) => setComputer(event.target.value)}><option value="">Choose a computer</option>{runOptions.computers.map((entry) => <option key={entry.id} value={entry.id}>{entry.name}</option>)}</select></label> : <label className="field">Name of the computer Lys runs on<input name="computer_name" value={computerName} required autoComplete="off" onChange={(event) => { setNamedComputer(true); setComputerName(event.target.value); }} /><span className="hint">{hostName ? name.trim() || 'This agent' : ''}{hostName ? ' runs there. It calls itself ' + hostName + '; change the name if you like.' : 'Type a name for the computer Lys runs on; this agent runs there.'}</span></label> : null}
        </div>
        {fields ? <div>{fields}</div> : null}
      </div>
    </fieldset>
    {!walk && capability.machineAdmission === false ? inUse.map((machine) => <p key={machine.id}>Choose which agents may run on {machine.name} in <a href={'#/network?computer=' + encodeURIComponent(machine.id)}>Network</a>, then start {name.trim() || 'this agent'} from its page.</p>) : null}
    {hold ? <ErrorWords problem={hold} /> : null}
    {inUse.length && (!onePress || hold || runOptions.runners.some((entry) => entry.state === 'unavailable')) ? <p>Which agents may run on each computer is chosen in Network. <a className="btn" href="#/network">Open Network</a></p> : null}
    {capability.problem ? <><p>Only “You” is offered under Who this agent answers to: Lys did not say whether it can register an agent under someone else, so the agent is added under you. You can change who it answers to afterwards, on its own page.</p><ErrorWords problem={capability.problem} /></>
      : !capability.answersTo ? <p>Only “You” is offered under Who this agent answers to: this Lys registers a new agent under the person adding it. To have it answer to someone else, add it here, then change who it answers to on its own page.</p> : null}
    {unsupported ? <p role="alert" className="why-not">{unsupported}</p> : null}
    {invalidChoice ? <p role="alert" className="why-not">{invalidChoice}</p> : null}
    {runOptions.loading && (!runOptions.runners.some((entry) => entry.state === 'asking') || runOptions.computers.length > 0) ? <p role="status">Reading the choices for running this agent…</p> : null}
    {runOptions.problem ? <p role="alert" className="why-not">{runnerProblemReason(runOptions.problem)} <a href="#/network">Open Network</a></p> : null}
    <div aria-live="polite">{runOptions.runners.map((entry) => entry.state === 'confirmed' ? null : <p key={entry.machine.id}>{entry.machine.name}{entry.state === 'unavailable' ? ': not available: ' + entry.reason : ': asking its runner…'}</p>)}</div>
    {needsComputer ? <p><a href="#/network">{CONNECT_COMPUTER}</a></p> : null}
    {saved.error ? saved.both
      ? <section className="why-not" role="alert" aria-label="Saved requests"><p>This browser kept two earlier requests without a confirmed answer: a registration, and a request to add and run an agent. Until both are settled, this form stays locked.</p>
        <p><Act symbol="retry" name="Check the saved registration first" word="Check the saved registration" tone="primary" onClick={() => restart(true)} /></p>
        <p className="note">It is sent again exactly as it was saved, so it never adds a second agent. The add-and-run request stays kept and is checked next.</p></section>
      : <ClearSavedRequest what="registration of an agent" keys={unreadableRecords(key, runKey, me.person.id)} cleared={() => restart(false)}
        check={<>Look for it in <a href="#/people">People and agents</a> first. If the agent is there, it was added; if not, clear this and add it again.</>} />
      : null}
    {saved.error ? <ErrorWords problem={saved.error} /> : null}
    {!saved.error && refusal ? refusal instanceof AddAndRunFailure ? <><p role="alert" className="why-not">{refusal.message}</p><ErrorWords problem={refusal.problem} /></> : <ErrorWords problem={refusal} /> : null}
    {walk ? <p role="status">Lys has not finished {addAndRunStep(walk)}. {hold ? 'Its original requests are kept.' : 'Its original requests are saved; this button continues them.'}</p> : null}
    {onePress && !walk && profileProblem ? <p className="why-not" role="alert">{profileProblem}</p> : null}
    {pending ? <p role="status">{pending.activated ? 'Your agent was added. Its team membership still needs confirmation.' : pending.agent ? 'Your agent was registered. Activation still needs confirmation.' : 'The registration has no confirmed answer yet.'} The original request is saved. Try that same request again without adding another agent.</p> : null}
    {/* The button is never greyed without its reason beside it: the reasons not already said above are said here. */}
    {!sending && !hold && !unavailable && !saved.error && !name.trim() ? <p className="why-not" role="alert">Give this agent a name.</p>
      : !sending && !hold && !unavailable && !saved.error && onePress && !walk && !profileProblem && !computerReady ? <p className="why-not" role="alert">{inUse.length ? 'Choose the computer this agent runs on.' : 'Give the computer Lys runs on a name.'}</p> : null}
    {onePress && !hold && !runsWith(settings) ? <p className="note add-agent-later">Without a folder, {name.trim() || 'this agent'} is added and allowed on {runComputerName || 'the chosen computer'} now, and its page asks for the folder before it starts.</p> : null}
    <p><Act symbol="add" name={hold ? 'Saved request held' : onePress ? 'Add ' + (name.trim() || 'agent') + (runsWith(settings) ? ' and run it on ' : ' to ') + (runComputerName || 'the chosen computer') : sending ? 'Adding…' : pending ? 'Continue adding this agent' : 'Add agent'} word={hold ? 'Held' : onePress ? runsWith(settings) ? 'Add and run' : 'Add' : sending ? 'Adding…' : pending ? 'Continue' : 'Add'} tone="primary" type="submit" disabled={sending || Boolean(hold) || runUnavailable || unavailable || !name.trim() || Boolean(saved.error) || (!pending && !walk && Boolean(taken || invalidChoice)) || (onePress && !walk && (Boolean(profileProblem) || !computerReady))} /></p>
  </form>;
  return onePress && runOptions.choices ? <ProfileFields profile={walk ? profileFromSettings(walk.settings) : null} choices={runOptions.choices} firstRun computerName={walk ? placementComputer(walk.placement).name : computerName} computer={walk ? placementComputer(walk.placement).id : selectedComputer?.id ?? ''}
    newComputer={walk ? walk.placement.kind === 'new' : !inUse.length} more={moreRegistration}
    pin={walk ? undefined : (machine) => { if (runOptions.computers.some((entry) => entry.id === machine)) setComputer(machine); }} render={renderForm} /> : renderForm(null);
}

function ReadyForm({ me, people, teams, search, capability, grants }: { me: MeView; people: PeopleView; teams: Team[]; search: string; capability: AddAndRunCapability; grants: GrantChoices }) {
  const [runOptions, setRunOptions] = useState<FirstRunOptions | null>(null);
  // Each restart reads the saved requests afresh in a new form; `later` holds a saved add-and-run back for one round.
  const [round, setRound] = useState({ count: 0, later: false });
  useEffect(() => {
    let live = true;
    const receive = (options: FirstRunOptions) => { if (live) setRunOptions(options); };
    void firstRunChoices(capability, me.person.id, people.scope === 'directory', receive).then(receive);
    return () => { live = false; };
  }, [capability, me.person.id, people.scope]);
  return runOptions ? <Form key={round.count} me={me} people={people} teams={teams} search={search} capability={capability} runOptions={runOptions} grants={grants} later={round.later} restart={(later) => setRound((prior) => ({ count: prior.count + 1, later }))} /> : <p role="status">Reading computers and their runners…</p>;
}

export function AddAgent() {
  const location = useLocation();
  const load = useLoad(async () => {
    const [me, people, teams, capability, grants] = await Promise.all([api.me(), api.people(), readTeams(), addAndRunCapability(), readGrantChoices()]);
    return { me, people, teams, capability, grants };
  }, 'add-agent');
  return <div className="page"><PageHeader breadcrumb={<a href="#/people">People and agents</a>} title="Add an agent" subtitle="An agent is an AI that works for a person. Name it, choose who it answers to and what it may use, then add it." />
    <Gate load={load} title="Add an agent" ok={(data) => <ReadyForm key={location.search} me={data.me} people={data.people} teams={data.teams} search={location.search} capability={data.capability} grants={data.grants} />} />
  </div>;
}
