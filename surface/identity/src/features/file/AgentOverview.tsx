import { useState } from 'react';
import { api, Refused, request, useLoad } from '../../api';
import type { Load } from '../../api';
import type { ReactNode } from 'react';
import type { AgentView } from '../../generated';
import type { NetworkView } from '../network/contract';
import { NetworkRead, oneNetworkRead } from '../network/shared-read';
import { readChoices } from '../provisioning/choices';
import type { Program } from '../provisioning/choices';
import { ProfileFields } from '../provisioning/ProfileEditor';
import { SaveSettings } from '../provisioning/SaveSettings';
import type { ProvisioningAnswer } from '../provisioning/Provisioning';
import type { Team } from '../teams/contract';
import { AgentTeams } from '../teams/TeamActions';
import { Pill } from '../people/Pill';
import { AgentRun } from '../team/AgentRun';
import './agent-overview.css';
import { Act } from '../../shell/Act';

function reading<T>(load: Load<T>, subject: string) {
  if (load.status === 'loading') return <span className="dim">Reading {subject}…</span>;
  if (load.status === 'refused') return <span className="why-not" role="alert">Lys could not read {subject} just now. The reason is below.</span>;
  return null;
}

function mismatch(name: string, reason: string): never {
  throw new Refused(0, { refusal: name, reason });
}

async function readProfile(id: string) {
  const answer = await request<ProvisioningAnswer>('/agents/' + encodeURIComponent(id) + '/provisioning');
  if (answer.agent !== id || !('profile' in answer)) mismatch('AgentSettingsMismatch', 'The saved settings did not answer this agent.');
  const profile = answer.profile;
  if (profile !== null && (!profile || !Array.isArray(profile.model_access) || profile.model_access.some((model) => typeof model !== 'string' || !model)
    || (profile.runs_on !== undefined && typeof profile.runs_on !== 'string'))) {
    mismatch('AgentSettingsUnreadable', 'The saved computer and model could not be read.');
  }
  return profile;
}

async function readTeams() {
  const answer = await request<{ teams: Team[] }>('/teams');
  if (!Array.isArray(answer.teams) || answer.teams.some((team) => !team || typeof team.id !== 'string' || typeof team.name !== 'string'
    || !Array.isArray(team.members) || team.members.some((member) => typeof member !== 'string') || !['active', 'retired'].includes(team.state))) {
    mismatch('AgentTeamsUnreadable', 'The service did not answer valid team memberships.');
  }
  return answer.teams;
}

async function readNetwork(read: () => Promise<NetworkView>) {
  const answer = await read();
  if (!Array.isArray(answer.machines) || answer.machines.some((machine) => !machine || typeof machine.id !== 'string' || typeof machine.name !== 'string'
    || !['in_use', 'retired'].includes(machine.state) || !(machine.runtime === null || typeof machine.runtime === 'string')
    || !Array.isArray(machine.may_run) || machine.may_run.some((entry) => !entry || typeof entry.id !== 'string')
    || (machine.may_run_roles !== undefined && (!Array.isArray(machine.may_run_roles) || machine.may_run_roles.some((role) => typeof role !== 'string'))))) {
    mismatch('AgentComputerUnreadable', 'The service did not answer computer names.');
  }
  return answer.machines;
}

async function readPrograms() {
  const answer = await request<{ programs: Program[] }>('/harnesses');
  if (!Array.isArray(answer.programs) || answer.programs.some((program) => !program || !Array.isArray(program.models)
    || program.models.some((model) => !model || typeof model.id !== 'string' || typeof model.label !== 'string'))) {
    mismatch('AgentModelsUnreadable', 'The service did not answer the models its programs list.');
  }
  return answer.programs;
}

/** A saved model by the name its program gives it; one no program lists is said to be unlisted, never shown as a bare id. */
function modelName(programs: Program[], id: string): string {
  const listed = programs.flatMap((program) => program.models).find((model) => model.id === id);
  return listed ? listed.label : id + ' (no program lists this model now)';
}

/** The agent's run and its saved choices, with every failed read named. The model and the computer are changed here with the settings form's own fields and saver; limits, access and the rest of its settings are the tabs beside this one. */
export function AgentOverview({ agent, details, added, rename }: { agent: AgentView; details: (problems: Refused[]) => ReactNode;
  /** The day the agent was added, and the act that opens its name's form: both stood in the page's head until 5 October 2026. */
  added?: string; rename?: () => void }) {
  const [revision, setRevision] = useState(0);
  const reload = () => setRevision((value) => value + 1);
  const profile = useLoad(() => readProfile(agent.id), 'agent-about-profile:' + agent.id + ':' + revision);
  const [teamRevision, setTeamRevision] = useState(0);
  const me = useLoad(api.me, 'agent-about-me');
  const teams = useLoad(readTeams, 'agent-about-teams:' + agent.id + ':' + teamRevision);
  const [networkRead] = useState(oneNetworkRead);
  const network = useLoad(() => readNetwork(networkRead), 'agent-about-computers:' + agent.id);
  const programs = useLoad(readPrograms, 'agent-about-programs:' + agent.id);
  const choices = useLoad(() => readChoices(), 'agent-about-choices:' + agent.id);
  const people = useLoad(api.people, 'agent-about-people');
  const computer = profile.status === 'ok' ? profile.data?.runs_on : undefined;
  const machine = network.status === 'ok' ? network.data.find((entry) => entry.id === computer) : undefined;
  const saved = profile.status === 'ok' ? profile.data?.model_access ?? [] : [];
  const models = programs.status === 'ok' ? saved.map((id) => modelName(programs.data, id)).join(', ') : '';
  const problems = [profile, teams, network, programs, choices, people, me].flatMap((load) => load.status === 'refused' ? [load.refused] : []);
  return <NetworkRead.Provider value={networkRead}><AgentRun key={agent.id} entry={{ id: agent.id, display_name: agent.display_name, state: agent.state, kind: 'agent', role: agent.role, person: agent.person }} />
  <section className="agent-overview" aria-label="About this agent">
    <dl className="facts agent-facts">
      <dt>Name</dt><dd>{agent.display_name}{rename ? <>{' '}<Act symbol="edit" name="Edit name" className="small" data-act="rename" onClick={rename} /></> : null}</dd>
      {added ? <><dt>Added</dt><dd>{added}</dd></> : null}
      <dt>Answers to</dt><dd><Pill x={agent.person} />{agent.needs_new_person ? <p className="why-not">{agent.person.state}: needs a new person before its access can be renewed.</p> : null}</dd>
      <dt>Team</dt><dd>{teams.status !== 'ok' ? reading(teams, 'teams') : me.status !== 'ok' ? reading(me, 'who is signed in')
        : <AgentTeams key={teamRevision} agent={agent.id} name={agent.display_name} teams={teams.data} person={me.data.person.id} login={me.data.signed_in} administrator={people.status === 'ok' && people.data.scope === 'directory'} changed={() => setTeamRevision((value) => value + 1)} />}</dd>
      {profile.status === 'ok' && profile.data?.harness && choices.status === 'ok' && people.status === 'ok'
        ? <><dt>Model and computer</dt><dd><ProfileFields brief key={profile.data.version} profile={profile.data} choices={choices.data} agent={agent.id} canChoose={people.data.scope === 'directory'} render={(fields, settings, refusal) => <>
          {fields}<SaveSettings agent={agent.id} profile={profile.data} settings={settings} refusal={refusal} canSave={people.data.scope === 'directory'} people={people.data} machines={choices.data.machines} saved={reload} brief />
        </>} /></dd></>
        : <>
      <dt>Computer</dt><dd>{profile.status !== 'ok' ? reading(profile, 'saved settings') : !computer ? 'Choose a computer when you start' : network.status !== 'ok' ? reading(network, 'computers') : machine ? <a href="#/network">{machine.name}</a> : <span className="why-not">The saved computer is no longer visible. Choose an available computer in Start.</span>}<p className="note">Saved choice for the next start.</p></dd>
      <dt>Model</dt><dd>{profile.status !== 'ok' ? reading(profile, 'saved settings') : !saved.length ? 'Choose a model when you start' : programs.status !== 'ok' ? reading(programs, 'model names') : models}<p className="note">Saved choice for the next start.</p></dd>
        </>}
    </dl>
  </section>{details(problems)}</NetworkRead.Provider>;
}
