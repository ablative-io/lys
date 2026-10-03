/** A stopped agent, started in place with one press. Where it runs is a saved setting: the computer its settings name, else the one computer allowed to run it; only when several are allowed and none is saved is each offered, as its own Start. What stops a start is said before the press, in one sentence with the one button that fixes it, and the button is never greyed without its reason beside it. */
import { useEffect, useRef, useState } from 'react';
import { Refused, api, operationId, request, useLoad } from '../../api';
import type { Machine, NetworkView } from '../network/contract';
import type { Entry } from '../people/directory';
import type { ProvisioningAnswer } from '../provisioning/Provisioning';
import { readRoles } from '../roles/AssignedRoles';
import { CannotStart } from '../runtime/CannotStart';
import type { StartRefusal } from '../runtime/CannotStart';
import { pendingStartOf, profileRequest, startRequest } from '../runtime/StartAgent';
import type { Pending } from '../runtime/StartAgent';

/** A refusal as the service named it; a failure the screen found itself keeps the name it was thrown with. */
function refusalOf(error: unknown): StartRefusal {
  if (error instanceof Refused) return { refusal: error.refusal.refusal, reason: error.message };
  const words = error instanceof Error ? error.message : String(error);
  const named = /^([A-Z][A-Za-z]+): (.*)$/s.exec(words);
  return named ? { refusal: named[1], reason: named[2] } : { refusal: 'Unexpected', reason: words };
}

/** The service answered no: nothing of this request is outstanding. Anything else may have been carried out. */
const answeredNo = (error: unknown) => error instanceof Refused && error.status >= 400 && error.status < 500;

export function Start({ entry, me, admin, changed, settings }: {
  entry: Entry; me: string; admin: boolean; changed: () => void; settings: () => void;
}) {
  const agent = entry.id;
  const name = entry.display_name;
  const prefix = '/agents/' + encodeURIComponent(agent);
  const key = 'lys.pending.agent-start.' + me + '.' + agent;
  const [round, setRound] = useState(0);
  const load = useLoad(async () => {
    const [identity, network, roles, provisioning] = await Promise.all([
      api.agent(agent), request<NetworkView>('/network'), readRoles(), request<ProvisioningAnswer>(prefix + '/provisioning'),
    ]);
    return { state: identity.state, machines: network.machines, roles: roles.roles, profile: provisioning.profile };
  }, 'team-start:' + agent + ':' + round);
  const [refusal, setRefusal] = useState<StartRefusal | null>(null);
  const [ended, setEnded] = useState('');
  const [busy, setBusy] = useState(false);
  const working = useRef(false);
  const wanted = useRef(false);

  const data = load.status === 'ok' ? load.data : null;
  const profile = data?.profile ?? null;
  const held = data ? data.roles.filter((role) => role.holders.some((holder) => holder.holder === agent && holder.state === 'holding')).map((role) => role.id) : [];
  const allowed: Machine[] = data ? data.machines.filter((machine) => machine.state === 'in_use' && machine.runtime !== null &&
    (machine.may_run.some((holder) => holder.id === agent) || Boolean(machine.may_run_roles?.some((role) => held.includes(role))))) : [];
  const saved = allowed.find((machine) => machine.id === profile?.runs_on);
  const choices = saved ? [saved] : allowed;
  const mayReview = admin || entry.person?.id === me;

  /** What is known to stop a start before any press, as the service would name it. */
  const known: StartRefusal | null = !data ? null
    : data.state !== 'active' ? { refusal: 'AgentNotActive', reason: 'the agent is ' + data.state + ' and only an active agent is started' }
    : !profile?.harness ? null
    : !profile.reviewed_by && !mayReview ? { refusal: 'NotAdmitted', reason: (entry.person?.display_name ?? 'The person it answers to') + ' or an administrator must approve the settings of ' + name + ' before it starts.' }
    : !profile.working_folder ? { refusal: 'WorkingFolderUnnamed', reason: 'the agent has no working folder' }
    : !allowed.length ? { refusal: 'MachineUnavailable', reason: 'no computer with Lys running on it is allowed to run this agent' }
    : null;
  const noProgram = Boolean(data) && data?.state === 'active' && !profile?.harness;

  const run = async (computer: string) => {
    if (working.current || !profile) return;
    working.current = true; setBusy(true); setRefusal(null); setEnded('');
    try {
      const review = (version: number, machine: string): Pending => ({ stage: 'review', path: prefix + '/provisioning/' + version + '/review', body: { operation: operationId() }, machine, version });
      const start = (version: number, machine: string): Pending => ({ stage: 'start', path: prefix + '/start-command', body: { machine, operation: operationId() }, machine, version });
      // A request this tab already sent and never saw answered is sent again, the same one, before anything new.
      const raw = sessionStorage.getItem(key);
      let current: Pending = raw === null
        ? profile.reviewed_by ? start(profile.version, computer) : review(profile.version, computer)
        : pendingStartOf(JSON.parse(raw), key, prefix);
      for (;;) {
        if (current.stage === 'review' && !mayReview) throw new Error('NotAdmitted: The person it answers to or an administrator must approve its settings before it starts.');
        sessionStorage.setItem(key, JSON.stringify(current));
        if (current.stage === 'start') {
          const receipt = await startRequest(agent, current);
          sessionStorage.removeItem(key);
          if (receipt.runner?.state === 'running') changed(); else setEnded(receipt.session);
          return;
        }
        const confirmed = await profileRequest(agent, current);
        current = current.stage === 'profile' && !confirmed.profile.reviewed_by ? review(confirmed.version, current.machine) : start(confirmed.version, current.machine);
      }
    } catch (error) {
      if (answeredNo(error) || error instanceof SyntaxError) sessionStorage.removeItem(key);
      setRefusal(refusalOf(error));
    } finally { working.current = false; setBusy(false); }
  };

  /** After a fix: read what this agent needs again, then send the start without a second press. The page around it is not reloaded, so this pane stays where it is. */
  const again = () => { setRefusal(null); setEnded(''); wanted.current = true; setRound((value) => value + 1); };
  useEffect(() => {
    if (!wanted.current || load.status === 'loading') return;
    wanted.current = false;
    if (load.status === 'ok' && !known && !noProgram && choices.length === 1) void run(choices[0].id);
  }, [load]);

  if (load.status === 'loading') return wanted.current ? <p role="status" className="team-start-line">Starting {name}…</p> : null;
  if (load.status === 'refused') return <div className="team-start" role="alert">
    <p className="team-start-line">Lys could not read what {name} needs to start. {load.refused.message} <small className="refusal-name">{load.refused.refusal.refusal}</small></p>
    <div className="team-start-acts"><button type="button" className="btn primary" onClick={() => setRound((value) => value + 1)}>Try again</button></div>
  </div>;
  const stopped = refusal ?? known;
  return <div className="team-start">
    <p className="team-start-line">{name} is not running.</p>
    {stopped ? <CannotStart agent={agent} name={name} refusal={stopped} again={again} /> : null}
    {!stopped && noProgram ? <div role="alert" className="cannot-start">
      <p>{name} has no program chosen yet.</p>
      <button type="button" className="btn primary" onClick={settings}>Choose its program</button>
    </div> : null}
    {!stopped && ended ? <p role="alert">{name} started and stopped straight away. <a href={'#/runtime/' + encodeURIComponent(ended)}>See what it printed</a></p> : null}
    {!stopped && !noProgram ? <div className="team-start-acts">{choices.map((computer) => <button key={computer.id} type="button" className="btn primary" disabled={busy} onClick={() => { void run(computer.id); }}>
      {busy ? 'Starting…' : choices.length === 1 ? 'Start' : 'Start on ' + computer.name}
    </button>)}</div> : null}
    {!stopped && !noProgram && choices.length === 1 ? <p className="dim team-start-where">On {choices[0].name}, in the folder {profile?.working_folder}.</p> : null}
  </div>;
}
