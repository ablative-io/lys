/** The requests a start is made of, each kept until its answer is confirmed: a settings save, its approval, and the start itself. A receipt that does not name the request it answers is refused by name, and the request stays kept. */
import { request } from '../../api';
import type { ProvisioningAnswer, ProvisioningProfile } from '../provisioning/Provisioning';

export interface StartAnswer {
  agent: string; machine: string; runtime: string; session: string; provisioning_version: number; harness: string;
  handles: { env: string; id: string; secret: string }[]; template: string; template_sha256: string; command: string; left_out: string[]; executed: false;
  runner?: { session: string; state: 'running' | 'ended'; pid: number | null; started_at: number };
}
export interface Pending {
  stage: 'profile' | 'review' | 'start'; path: string; body: Record<string, unknown>;
  machine: string; version: number; legacyKey?: string;
}
function same(left: unknown, right: unknown): boolean {
  if (left === right) return true;
  if (!left || !right || typeof left !== 'object' || typeof right !== 'object') return false;
  if (Array.isArray(left) || Array.isArray(right)) return Array.isArray(left) && Array.isArray(right) && left.length === right.length && left.every((value, index) => same(value, right[index]));
  const keys = Object.keys(left);
  return keys.length === Object.keys(right).length && keys.every((name) => Object.hasOwn(right, name) && same(member(left, name), member(right, name)));
}
function setting(name: string, value: unknown): unknown {
  return name === 'permissions' && value && typeof value === 'object'
    ? { allow: [], deny: [], ask: [], additional_directories: [], ...value }
    : value;
}
function fail(name: string, words: string): never { throw new Error(name + ': ' + words); }
function record(value: unknown): value is Record<string, unknown> { return Boolean(value) && typeof value === 'object' && !Array.isArray(value); }
function member(value: unknown, name: string): unknown { return record(value) ? value[name] : undefined; }

export function pendingStartOf(value: unknown, key: string, prefix: string): Pending {
  if (!record(value) || !record(value.body)) return fail('PendingStartUnreadable', 'The retained start is not a request.');
  if (value.stage !== 'profile' && value.stage !== 'review' && value.stage !== 'start') return fail('PendingStartUnreadable', 'The retained request has no recognized stage.');
  if (typeof value.version !== 'number' || !Number.isInteger(value.version) || value.version < 0) return fail('PendingStartUnreadable', 'The retained request has no valid version.');
  const path = value.stage === 'profile' ? prefix + '/provisioning' : value.stage === 'review' ? prefix + '/provisioning/' + value.version + '/review' : prefix + '/start-command';
  if (value.path !== path || typeof value.body.operation !== 'string' || !/^op-[0-9a-f]{32}$/.test(value.body.operation) || typeof value.machine !== 'string' || !value.machine) return fail('PendingStartUnreadable', 'The retained start must be resolved before another request.');
  if (value.stage === 'start' && value.body.machine !== value.machine) return fail('PendingStartUnreadable', 'The retained start names two different computers.');
  if (value.stage === 'profile' && value.body.from_version !== value.version) return fail('PendingStartUnreadable', 'The retained save names two different versions.');
  if (value.legacyKey !== undefined && typeof value.legacyKey !== 'string') return fail('PendingStartUnreadable', 'The retained request names an unreadable record.');
  const legacyKey = value.legacyKey;
  if (legacyKey !== undefined && !['provisioning', 'profile-review', 'start'].some((kind) => legacyKey === key.replace('agent-start', kind))) return fail('PendingStartUnreadable', 'The retained request names an unrelated record.');
  return { stage: value.stage, path, body: value.body, machine: value.machine, version: value.version, legacyKey };

}

export async function startRequest(agent: string, current: Pending): Promise<StartAnswer> {
  const receipt: StartAnswer = await request<StartAnswer>(current.path, current.body);
  if (receipt.agent !== agent || receipt.machine !== current.machine || receipt.session !== current.body.operation || receipt.executed !== false || typeof receipt.command !== 'string' || !receipt.command || !Array.isArray(receipt.left_out)) fail('StartReceiptMismatch', 'The start answer did not confirm the retained request.');
  if (receipt.provisioning_version !== current.version) fail('StartVersionChanged', 'The runner start names a different profile version; check that session before another start.');
  const runner = receipt.runner;
  if (!runner) fail('RunnerStartUnconfirmed', 'Lys admitted the start, but no runner ran it. The same request is retained.');
  if (runner.session !== receipt.session || !['running', 'ended'].includes(runner.state)) fail('StartReceiptMismatch', 'The runner answer did not name this session and its state.');
  return receipt;
}

export async function profileRequest(agent: string, current: Pending): Promise<{ profile: ProvisioningProfile; version: number; notice: string }> {
  const receipt: ProvisioningAnswer = await request<ProvisioningAnswer>(current.path, current.body);
  const savedProfile = receipt.profile;
  const recorded = receipt.recorded;
  if (receipt.agent !== agent || !savedProfile || !recorded) fail('ProfileReceiptMismatch', 'The answer did not confirm the retained profile request.');
  if (current.stage === 'profile') {
    const version = current.version + 1;
    if (recorded.operation !== current.body.operation || recorded.version !== version || savedProfile.version !== version || savedProfile.operation !== current.body.operation) fail('ProfileReceiptMismatch', 'The saved version does not match this request, or a newer version replaced it.');
    if (Object.entries(current.body).some(([name, value]) => !['operation', 'from_version', 'note'].includes(name) && !same(setting(name, value), setting(name, member(savedProfile, name))))) fail('ProfileReceiptMismatch', 'The saved settings differ from this retained request.');
    return { profile: savedProfile, version, notice: '' };
  }
  if (recorded.version !== current.version || savedProfile.version !== current.version || !savedProfile.reviewed_by || !/^op-[0-9a-f]{32}$/.test(recorded.operation)) fail('ProfileReceiptMismatch', 'The review did not confirm the current profile version.');
  return { profile: savedProfile, version: current.version, notice: recorded.operation === current.body.operation ? 'Version ' + current.version + ' of these settings is approved.' : 'Version ' + current.version + ' was already approved by someone else. This did not replace that approval.' };
}

/** Whether the settings on the form differ from the saved profile; the note is not a setting. A profile with no program has nothing saved to keep. */
export function changedFrom(profile: ProvisioningProfile | null, settings: Record<string, unknown>): boolean {
  return !profile?.harness || Object.entries(settings).some(([name, value]) => name !== 'note' && !same(setting(name, value), setting(name, member(profile, name))));
}
