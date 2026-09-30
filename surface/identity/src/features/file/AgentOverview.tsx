import { Refused, request, useLoad } from '../../api';
import type { Load } from '../../api';
import type { ReactNode } from 'react';
import type { AgentView } from '../../generated';
import type { NetworkView } from '../network/contract';
import type { ProvisioningAnswer } from '../provisioning/Provisioning';
import type { RuntimeSession } from '../runtime/RuntimeSessions';
import type { Team } from '../teams/contract';
import { Pill } from '../people/Pill';
import { clock } from './time';
import './agent-overview.css';

function reading<T>(load: Load<T>, subject: string) {
  if (load.status === 'loading') return <span className="dim">Reading {subject}…</span>;
  if (load.status === 'refused') return <span className="why-not" role="alert">Lys could not read {subject} just now. Open Details for the reason.</span>;
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

async function readNetwork() {
  const answer = await request<NetworkView>('/network');
  if (!Array.isArray(answer.machines) || answer.machines.some((machine) => !machine || typeof machine.id !== 'string' || typeof machine.name !== 'string'
    || !['in_use', 'retired'].includes(machine.state) || !(machine.runtime === null || typeof machine.runtime === 'string')
    || !Array.isArray(machine.may_run) || machine.may_run.some((entry) => !entry || typeof entry.id !== 'string')
    || (machine.may_run_roles !== undefined && (!Array.isArray(machine.may_run_roles) || machine.may_run_roles.some((role) => typeof role !== 'string'))))) {
    mismatch('AgentComputerUnreadable', 'The service did not answer computer names.');
  }
  return answer.machines;
}

async function readSessions(id: string) {
  const answer = await request<{ sessions: RuntimeSession[] }>('/agents/' + encodeURIComponent(id) + '/runtime/sessions');
  if (!Array.isArray(answer.sessions)) mismatch('RuntimeReportUnreadable', 'The runner did not answer a session list.');
  for (const session of answer.sessions) {
    if (!session || session.agent !== id) mismatch('RuntimeReportMismatch', 'A runner report did not name this agent.');
    if (typeof session.session !== 'string' || typeof session.machine !== 'string'
      || !(session.machine_name === null || typeof session.machine_name === 'string')
      || !['running', 'stopped', 'unconfirmed'].includes(session.shown) || !Number.isFinite(session.last_report_at)
      || (session.shown === 'stopped' && (!session.stopped || typeof session.stopped.confirmation !== 'string' || !session.stopped.confirmation))) {
      mismatch('RuntimeReportUnreadable', 'The runner did not answer a valid state and report time.');
    }
  }
  return answer.sessions;
}

function Running({ load }: { load: Load<RuntimeSession[]> }) {
  if (load.status !== 'ok') return <>{reading(load, 'runner reports')}<p className="note">Running state is unknown until the runner answers.</p></>;
  const sessions = load.data;
  if (!sessions.length) return <>No runner has reported a session. <span className="note">Running state is unknown.</span></>;
  const latest: Partial<Record<RuntimeSession['shown'], RuntimeSession>> = {};
  for (const session of sessions) {
    const previous = latest[session.shown];
    if (!previous || previous.last_report_at < session.last_report_at) latest[session.shown] = session;
  }
  return <>
    <span className={latest.running ? 'agent-running' : ''}>{latest.running ? 'Running, as its runner last reported' : latest.unconfirmed ? 'Running state is unconfirmed' : 'Stopped, as its runner confirmed'}</span>
    <ul className="agent-reports">{Object.values(latest).map((session) => <li key={session.session}>
      <a href={'#/runtime/' + encodeURIComponent(session.session)}>{session.machine_name || 'Reported computer'}</a>
      {' · '}{session.shown === 'running' ? 'reported running' : session.shown === 'stopped' ? 'confirmed stopped' : 'not confirmed'}
      {' · '}{clock(session.last_report_at)}
      {session.stop_asked_at && !session.stopped ? <p className="why-not">A stop was requested. The runner has not confirmed it ended.</p> : null}
    </li>)}</ul><a className="note" href={'#/file/' + encodeURIComponent(sessions[0].agent ?? '') + '/sessions'}>See all runner reports</a>
  </>;
}

/** Saved choices and runner observations stay distinct, with every failed read named. */
export function AgentOverview({ agent, details }: { agent: AgentView; details: (problems: Refused[]) => ReactNode }) {
  const profile = useLoad(() => readProfile(agent.id), 'agent-about-profile:' + agent.id);
  const teams = useLoad(readTeams, 'agent-about-teams:' + agent.id);
  const network = useLoad(readNetwork, 'agent-about-computers:' + agent.id);
  const sessions = useLoad(() => readSessions(agent.id), 'agent-about-sessions:' + agent.id);
  const held = teams.status === 'ok' ? teams.data.filter((team) => team.state === 'active' && team.members.includes(agent.id)) : [];
  const computer = profile.status === 'ok' ? profile.data?.runs_on : undefined;
  const machine = network.status === 'ok' ? network.data.find((entry) => entry.id === computer) : undefined;
  const noComputer = network.status === 'ok' && !network.data.some((entry) => entry.state === 'in_use' && entry.runtime !== null
    && (entry.may_run.some((allowed) => allowed.id === agent.id) || Boolean(entry.may_run_roles?.length)));
  const problems = [profile, teams, network, sessions].flatMap((load) => load.status === 'refused' ? [load.refused] : []);
  return <><section className="agent-overview" aria-label="About this agent">
    <dl className="facts agent-facts">
      <dt>Answers to</dt><dd><Pill x={agent.person} />{agent.needs_new_person ? <p className="why-not">{agent.person.state}: needs a new person before its access can be renewed.</p> : null}</dd>
      <dt>Team</dt><dd>{teams.status !== 'ok' ? reading(teams, 'teams') : held.length ? held.map((team, index) => <span key={team.id}>{index ? ', ' : ''}<a href="#/teams">{team.name}</a></span>) : 'No team yet'}</dd>
      <dt>Computer</dt><dd>{profile.status !== 'ok' ? reading(profile, 'saved settings') : !computer ? 'Choose a computer when you start' : network.status !== 'ok' ? reading(network, 'computers') : machine ? <a href="#/network">{machine.name}</a> : <span className="why-not">The saved computer is no longer visible. Choose an available computer in Start.</span>}<p className="note">Saved choice; the runner reports below say where a session was seen.</p></dd>
      <dt>Model</dt><dd>{profile.status !== 'ok' ? reading(profile, 'saved settings') : profile.data?.model_access.length ? profile.data.model_access.join(', ') : 'Choose a model when you start'}<p className="note">Saved choice for the next start.</p></dd>
      <dt>Running</dt><dd><Running load={sessions} /></dd>
    </dl>
  </section><AgentNextSteps id={agent.id} name={agent.display_name} noComputer={noComputer} />{details(problems)}</>;
}

/** Each next act opens the existing form and never makes a change just by visiting. */
export function AgentNextSteps({ id, name, noComputer }: { id: string; name: string; noComputer: boolean }) {
  const base = '#/file/' + encodeURIComponent(id) + '/';
  return <nav className="agent-next" aria-label="Next steps">
    <a data-act="start" href={base + 'provisioning'}><strong>Start</strong><span>{noComputer ? 'No computer lets ' + name + ' run yet. Ask for it to be allowed on a computer before starting.' : 'Choose its computer and model, then start this agent.'}</span></a>
    <a href={base + 'budgets'}><strong>Set limits</strong><span>Set how much this agent may use and when it must stop.</span></a>
    <a href={base + 'access'}><strong>Give access</strong><span>Choose what this agent may reach from access you can give.</span></a>
  </nav>;
}
