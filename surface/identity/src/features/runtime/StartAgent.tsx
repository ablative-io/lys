import { useRef, useState } from 'react';
import { Refused, api, operationId, request, useLoad } from '../../api';
import { readRoles } from '../roles/AssignedRoles';
import type { Machine, NetworkView } from '../network/contract';
import { entries } from '../people/directory';
import type { ProvisioningAnswer, ProvisioningProfile } from '../provisioning/Provisioning';
import { Gate } from '../signin/Gate';

export interface StartAnswer {
  agent: string; machine: string; runtime: string; session: string; provisioning_version: number; harness: string;
  handles: { env: string; id: string; secret: string }[]; template: string; template_sha256: string; command: string; left_out: string[]; executed: false;
  runner?: { session: string; state: 'running' | 'ended'; pid: number | null; started_at: number };
}
interface Pending {
  stage: 'profile' | 'review' | 'start'; path: string; body: Record<string, unknown>;
  machine: string; version: number; legacyKey?: string;
}
function same(left: unknown, right: unknown): boolean {
  if (left === right) return true;
  if (!left || !right || typeof left !== 'object' || typeof right !== 'object') return false;
  if (Array.isArray(left) || Array.isArray(right)) return Array.isArray(left) && Array.isArray(right) && left.length === right.length && left.every((value, index) => same(value, right[index]));
  const keys = Object.keys(left);
  return keys.length === Object.keys(right).length && keys.every((name) => Object.hasOwn(right, name) && same((left as Record<string, unknown>)[name], (right as Record<string, unknown>)[name]));
}
const fail = (name: string, words: string): never => { throw new Error(name + ': ' + words); };

function permitted(machine: Machine, agent: string, held: string[]): boolean {
  return machine.state === 'in_use' && machine.runtime !== null &&
    (machine.may_run.some((entry) => entry.id === agent) || Boolean(machine.may_run_roles?.some((role) => held.includes(role))));
}
function retained(key: string, prefix: string): Pending | null {
  const raw = sessionStorage.getItem(key);
  if (raw === null) return null;
  const parsed: unknown = JSON.parse(raw);
  if (!parsed || typeof parsed !== 'object' || Array.isArray(parsed)) return fail('PendingStartUnreadable', 'The retained start is not a request.');
  const value = parsed as Partial<Pending>;
  const path = value.stage === 'profile' ? prefix + '/provisioning' : value.stage === 'review' ? prefix + '/provisioning/' + value.version + '/review' : value.stage === 'start' ? prefix + '/start-command' : '';
  if (!value || !path || value.path !== path || !value.body || typeof value.body !== 'object' || Array.isArray(value.body)
    || typeof value.body.operation !== 'string' || !/^op-[0-9a-f]{32}$/.test(value.body.operation)
    || typeof value.machine !== 'string' || !value.machine || !Number.isInteger(value.version) || Number(value.version) < 0) {
    return fail('PendingStartUnreadable', 'The retained start must be resolved before another request.');
  }
  if (value.stage === 'start' && value.body.machine !== value.machine) return fail('PendingStartUnreadable', 'The retained start names two different computers.');
  if (value.stage === 'profile' && value.body.from_version !== value.version) return fail('PendingStartUnreadable', 'The retained save names two different versions.');
  return value as Pending;
}

export function StartAgent({ agent, profile, settings, refusal = '', canSave }: {
  agent: string; profile: ProvisioningProfile | null; settings?: Record<string, unknown>; refusal?: string; canSave?: boolean;
}) {
  const load = useLoad(async () => {
    const [me, people, network, roles] = await Promise.all([api.me(), api.people(), request<NetworkView>('/network'), readRoles()]);
    return { me, people, network, roles };
  }, 'launch-options:' + agent);
  return <Gate load={load} title="Start" ok={({ me, people, network, roles }) => {
    const identity = entries(people).find((entry) => entry.id === agent);
    const held = roles.roles.filter((role) => role.holders.some((holder) => holder.holder === agent && holder.state === 'holding')).map((role) => role.id);
    const machines = network.machines.filter((machine) => permitted(machine, agent, held));
    const mayReview = people.scope === 'directory' || identity?.person?.id === me.person.id;
    const problem = identity?.state !== 'active' ? 'AgentNotActive: turn this agent on before starting it.' : !machines.length ? 'MachineUnavailable: no computer with a Lys runner admits this agent.' : refusal;
    return <StartForm agent={agent} person={me.person.id} machines={machines} profile={profile} settings={settings} refusal={problem} canSave={canSave ?? people.scope === 'directory'} mayReview={mayReview} />;
  }} />;
}

function StartForm({ agent, person, machines, profile, settings, refusal, canSave, mayReview }: {
  agent: string; person: string; machines: Machine[]; profile: ProvisioningProfile | null;
  settings?: Record<string, unknown>; refusal: string; canSave: boolean; mayReview: boolean;
}) {
  const prefix = '/agents/' + encodeURIComponent(agent);
  const key = 'lys.pending.agent-start.' + person + '.' + agent;
  const preferred = machines.some((entry) => entry.id === profile?.runs_on) ? profile?.runs_on ?? '' : machines.length === 1 ? machines[0].id : '';
  const [initial] = useState(() => {
    try {
      const current = retained(key, prefix);
      if (current) return { pending: current, error: '' };
      const old = ['provisioning', 'profile-review', 'start'].map((kind) => ({ kind, key: 'lys.pending.' + kind + '.' + person + '.' + agent })).filter((entry) => sessionStorage.getItem(entry.key) !== null);
      if (old.length > 1) fail('EarlierChangeUnresolved', 'Several earlier requests are retained; their outcomes must be resolved individually.');
      if (!old.length) return { pending: null, error: '' };
      const entry = old[0];
      const raw: unknown = JSON.parse(sessionStorage.getItem(entry.key)!);
      if (!raw || typeof raw !== 'object' || !('path' in raw) || !('body' in raw) || !raw.body || typeof raw.body !== 'object' || Array.isArray(raw.body)) fail('PendingStartUnreadable', 'The earlier retained request is unreadable.');
      const body = raw.body as Record<string, unknown>;
      const stage = entry.kind === 'provisioning' ? 'profile' : entry.kind === 'profile-review' ? 'review' : 'start';
      const version = stage === 'profile' ? Number(body.from_version) : stage === 'review' && typeof raw.path === 'string' ? Number(raw.path.split('/').at(-2)) : profile?.version ?? 0;
      const expected = stage === 'profile' ? prefix + '/provisioning' : stage === 'review' ? prefix + '/provisioning/' + version + '/review' : prefix + '/start-command';
      if (raw.path !== expected || typeof body.operation !== 'string' || !/^op-[0-9a-f]{32}$/.test(body.operation) || !Number.isInteger(version) || version < 0) fail('PendingStartUnreadable', 'The earlier request does not name this agent and operation.');
      return { pending: { stage, path: expected, body, version, machine: stage === 'start' && typeof body.machine === 'string' ? body.machine : preferred, legacyKey: entry.key } as Pending, error: '' };
    }
    catch (error) { return { pending: null, error: String(error) }; }
  });
  const [pending, setPending] = useState(initial.pending);
  const [machine, setMachine] = useState(initial.pending?.machine || preferred);
  const [failure, setFailure] = useState(initial.error);
  const [busy, setBusy] = useState(false);
  const [answer, setAnswer] = useState<StartAnswer | null>(null);
  const working = useRef(false);
  const block = initial.error || refusal;
  const persist = (next: Pending) => { sessionStorage.setItem(key, JSON.stringify(next)); setPending(next); };
  const nextReview = (version: number, computer: string): Pending => ({ stage: 'review', path: prefix + '/provisioning/' + version + '/review', body: { operation: operationId() }, machine: computer, version });
  const nextStart = (version: number, computer: string): Pending => ({ stage: 'start', path: prefix + '/start-command', body: { machine: computer, operation: operationId() }, machine: computer, version });
  const run = async () => {
    if (working.current || answer || initial.error) return;
    working.current = true; setBusy(true); setFailure('');
    let current = pending;
    try {
      if (current && !current.machine) {
        if (!machines.some((entry) => entry.id === machine)) fail('MachineUnavailable', 'Choose the computer for this retained request.');
        current = { ...current, machine };
      }
      if (!current) {
        if (block) fail('StartUnavailable', block);
        if (!machines.some((entry) => entry.id === machine)) fail('MachineUnavailable', 'Choose a computer this agent may use.');
        const changed = settings && (!profile?.harness || Object.entries(settings).some(([name, value]) => name !== 'note' && !same(value, profile[name as keyof ProvisioningProfile])));
        if (changed) {
          if (!canSave) fail('NotAdmitted', 'An administrator must save this agent’s settings.');
          if (profile && !('session' in profile)) fail('ProfileReadIncomplete', 'Update Lys before changing this profile; its saved session settings are not returned.');
          current = { stage: 'profile', path: prefix + '/provisioning', body: { ...settings, operation: operationId(), from_version: profile?.version ?? 0 }, machine, version: profile?.version ?? 0 };
        } else {
          if (!profile?.harness) fail('ProgramUnavailable', 'This agent has no saved program.');
          current = profile.reviewed_by ? nextStart(profile.version, machine) : nextReview(profile.version, machine);
        }
      }
      while (current) {
        if (current.stage === 'review' && !mayReview) fail('NotAdmitted', 'The responsible person or an administrator must approve this profile.');
        persist(current);
        if (current.stage === 'start') {
          const receipt = await request<StartAnswer>(current.path, current.body);
          if (receipt.agent !== agent || receipt.machine !== current.machine || receipt.session !== current.body.operation || receipt.executed !== false || typeof receipt.command !== 'string' || !receipt.command || !Array.isArray(receipt.left_out)) fail('StartReceiptMismatch', 'The start answer did not confirm the retained request.');
          if (!receipt.runner) fail('RunnerStartUnconfirmed', 'Lys returned a command but no runner confirmed this agent started. The same request is retained.');
          if (receipt.runner.session !== receipt.session || !['running', 'ended'].includes(receipt.runner.state)) fail('StartReceiptMismatch', 'The runner answer did not name this session and its state.');
          if (current.legacyKey) sessionStorage.removeItem(current.legacyKey);
          sessionStorage.removeItem(key); setPending(null); setAnswer(receipt); return;
        }
        const receipt = await request<ProvisioningAnswer>(current.path, current.body);
        if (receipt.agent !== agent || !receipt.profile || !receipt.recorded) fail('ProfileReceiptMismatch', 'The answer did not confirm the retained profile request.');
        const legacyKey = current.legacyKey;
        if (current.stage === 'profile') {
          const version = current.version + 1;
          if (receipt.recorded.operation !== current.body.operation || receipt.recorded.version !== version || receipt.profile.version !== version || receipt.profile.operation !== current.body.operation) fail('ProfileReceiptMismatch', 'The saved version does not match this request, or a newer version replaced it.');
          current = receipt.profile.reviewed_by ? nextStart(version, current.machine) : nextReview(version, current.machine);
        } else {
          if (receipt.recorded.version !== current.version || receipt.profile.version !== current.version || !receipt.profile.reviewed_by || !/^op-[0-9a-f]{32}$/.test(receipt.recorded.operation)) fail('ProfileReceiptMismatch', 'The review did not confirm the current profile version.');
          current = nextStart(current.version, current.machine);
        }
        persist(current);
        if (legacyKey) sessionStorage.removeItem(legacyKey);
      }
    } catch (error) {
      // A refusal after a saved stage must not repeat the save with an obsolete version.
      setFailure(error instanceof Refused ? error.refusal.refusal + ': ' + error.message : String(error));
    } finally { working.current = false; setBusy(false); }
  };
  if (answer) return <div role="status"><p>{answer.runner?.state === 'running' ? 'Started on ' + (machines.find((entry) => entry.id === answer.machine)?.name ?? answer.machine) + '. Its Lys runner has it running.' : 'Started, and its Lys runner saw it end.'}</p>
    <a className="btn primary" href={'#/runtime/' + encodeURIComponent(answer.session)}>Open its terminal</a>
    {answer.left_out.length ? <><p>Not carried into this start:</p><ul>{answer.left_out.map((entry, index) => <li key={index}>{entry}</li>)}</ul></> : null}
  </div>;
  return <form onSubmit={(event) => { event.preventDefault(); void run(); }}>
    <label className="field">Computer this agent runs on<select name="machine" value={machine} disabled={busy || Boolean(pending?.machine)} onChange={(event) => setMachine(event.target.value)}>{machines.length !== 1 ? <option value="">Choose a computer</option> : null}{machines.map((entry) => <option key={entry.id} value={entry.id}>{entry.name}</option>)}</select></label>
    {block ? <p role="alert" className="why-not">{block}</p> : null}
    {pending ? <p role="status">This start has no confirmed answer. Its original request is retained. Press Start this agent to check that same request.</p> : null}
    {failure ? <p role="alert" className="why-not">{failure}</p> : null}
    <button type="submit" className="btn primary" disabled={busy || Boolean(initial.error) || (!pending && (Boolean(block) || !machine))}>Start this agent</button>
  </form>;
}
