/** A profile version is recorded with every member the start carries: the declared harness, each MCP server as an address or a command with its channel, and the settings file's permissions. */
import { useState } from 'react';
import type { FormEvent } from 'react';
import { operationId } from '../../api';
import { field } from '../people/RecordedForm';
import { useRoleChange } from '../roles/useRoleChange';
import { ChangeStatus } from '../roles/ChangeStatus';
import type { McpServer, Permissions, ProvisioningAnswer, ProvisioningProfile } from './Provisioning';

export const MODES = ['acceptEdits', 'auto', 'bypassPermissions', 'manual', 'dontAsk', 'plan'];
const lines = (data: FormData, name: string) => field(data, name).split('\n').map((line) => line.trim()).filter(Boolean);
type Draft = McpServer & { key: number; kind: 'address' | 'command' };

function server(data: FormData, draft: Draft): McpServer {
  const name = field(data, 'server-name-' + draft.key);
  const channel = field(data, 'server-channel-' + draft.key) === 'wake' ? 'wake' : 'off';
  if (!name) throw new Error('Each MCP server needs a name.');
  if (draft.kind === 'address') {
    const url = field(data, 'server-url-' + draft.key); const parsed = new URL(url);
    if (!['http:', 'https:'].includes(parsed.protocol) || parsed.username || parsed.password) throw new Error('MCP server ' + name + ' needs an HTTP or HTTPS address without an embedded username or password.');
    return { name, url, channel };
  }
  const program = field(data, 'server-program-' + draft.key); const cwd = field(data, 'server-cwd-' + draft.key);
  const envText = field(data, 'server-env-' + draft.key) || '{}';
  let env: unknown;
  try { env = JSON.parse(envText); } catch { throw new Error('The environment of MCP server ' + name + ' is not JSON.'); }
  if (!env || typeof env !== 'object' || Array.isArray(env)) throw new Error('The environment of MCP server ' + name + ' is not a JSON object.');
  if (!program) throw new Error('MCP server ' + name + ' needs the program it is started with.');
  let args: unknown;
  try { args = JSON.parse(field(data, 'server-args-' + draft.key) || '[]'); } catch { throw new Error('The arguments of MCP server ' + name + ' are not JSON.'); }
  if (!Array.isArray(args) || !args.every((arg) => typeof arg === 'string')) throw new Error('The arguments of MCP server ' + name + ' are not a JSON array of strings.');
  return { name, command: { program, args, ...(cwd ? { cwd } : {}), env: env as NonNullable<McpServer['command']>['env'] }, channel };
}

function permissions(data: FormData): Permissions | null {
  const given: Permissions = { allow: lines(data, 'perm-allow'), deny: lines(data, 'perm-deny'), ask: lines(data, 'perm-ask'), additional_directories: lines(data, 'perm-dirs') };
  const mode = field(data, 'perm-mode');
  if (mode) given.default_mode = mode;
  const empty = !mode && [given.allow, given.deny, given.ask, given.additional_directories].every((list) => !list?.length);
  return empty ? null : given;
}

export function ProfileEditor({ id, path, person, profile, changed }: { id: string; path: string; person: string; profile: ProvisioningProfile | null; changed: () => void }) {
  const [servers, setServers] = useState<Draft[]>((profile?.mcp_servers ?? []).map((value, key) => ({ ...value, key, kind: value.command ? 'command' : 'address' })));
  const [nextKey, setNextKey] = useState(servers.length);
  const [error, setError] = useState('');
  const change = useRoleChange<ProvisioningAnswer>('lys.pending.provisioning.' + person + '.' + id, path,
    (answer, body) => answer.agent === id && (answer.recorded ? answer.recorded.operation === body.operation && answer.recorded.version === Number(body.from_version) + 1 : answer.profile !== null && answer.profile.operation === body.operation && answer.profile.version === Number(body.from_version) + 1), changed);
  const kind = (key: number, next: Draft['kind']) => setServers((values) => values.map((entry) => entry.key === key ? { ...entry, kind: next } : entry));
  const submit = (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault(); if (change.blocked) return;
    try {
      const data = new FormData(event.currentTarget);
      const mcp_servers = servers.map((draft) => server(data, draft));
      if (new Set(mcp_servers.map((entry) => entry.name)).size !== mcp_servers.length) throw new Error('Give each MCP server a different name.');
      const program = field(data, 'harness-program'); const pkg = field(data, 'harness-package');
      if (Boolean(program) !== Boolean(pkg)) throw new Error('Declare the harness with both its program and its package, or leave both empty.');
      const harness = program ? { kind: field(data, 'harness-kind') === 'codex' ? 'codex' : 'claude_code', program, package: pkg } : null;
      setError('');
      change.submit({ operation: operationId(), from_version: profile?.version ?? 0, model_access: lines(data, 'model_access'), tools: lines(data, 'tools'), skills: lines(data, 'skills'), mcp_servers, instructions: field(data, 'instructions'), note: field(data, 'note'), harness, permissions: permissions(data) });
    } catch (failure) { setError(String(failure)); }
  };
  const held = profile?.permissions;
  return <form className="card recorded-form" aria-label="Record provisioning profile" onSubmit={submit}><h3>{profile ? 'Record a new version' : 'Set up a profile'}</h3>
    <fieldset disabled={change.blocked} style={{ border: 0, padding: 0 }}>
      <label className="field">Instructions<textarea name="instructions" defaultValue={profile?.instructions ?? ''} rows={5} placeholder="How this agent should carry out its work" /></label>
      <details><summary>Harness</summary><p>The build this agent is started with. A start is refused until one is declared.</p>
        <label className="field">Harness<select name="harness-kind" defaultValue={profile?.harness?.kind ?? 'claude_code'}><option value="claude_code">Claude Code</option><option value="codex">Codex</option></select></label>
        <label className="field">Program<input name="harness-program" defaultValue={profile?.harness?.program ?? ''} placeholder="/absolute/path/to/program" /></label>
        <label className="field">Package<input name="harness-package" defaultValue={profile?.harness?.package ?? ''} placeholder="The package the program is verified against" /></label>
      </details>
      <details><summary>Models, tools, skills and MCP servers</summary><p>One name per line. These declarations do not give permission to use a service.</p>
        <label className="field">Model access<textarea name="model_access" defaultValue={profile?.model_access.join('\n') ?? ''} /></label>
        <label className="field">Tools<textarea name="tools" defaultValue={profile?.tools.join('\n') ?? ''} /></label>
        <label className="field">Skills<textarea name="skills" defaultValue={profile?.skills.join('\n') ?? ''} /></label>
        {servers.map((draft) => <fieldset key={draft.key} aria-label={'MCP server ' + (draft.name || draft.key)}>
          <label className="field">Server name<input name={'server-name-' + draft.key} defaultValue={draft.name} required /></label>
          <label className="field">Started as<select value={draft.kind} onChange={(event) => kind(draft.key, event.target.value === 'command' ? 'command' : 'address')}><option value="address">An address</option><option value="command">A command</option></select></label>
          {draft.kind === 'address' ? <label className="field">Server address<input name={'server-url-' + draft.key} type="url" defaultValue={draft.url ?? ''} required /></label> : <>
            <label className="field">Program<input name={'server-program-' + draft.key} defaultValue={draft.command?.program ?? ''} required /></label>
            <label className="field">Arguments (a JSON array of strings, exactly as passed)<textarea name={'server-args-' + draft.key} defaultValue={JSON.stringify(draft.command?.args ?? [])} /></label>
            <label className="field">Directory<input name={'server-cwd-' + draft.key} defaultValue={draft.command?.cwd ?? ''} /></label>
            <label className="field">Environment (a JSON object; a secret as {'{"handle": "<secret name>"}'})<textarea name={'server-env-' + draft.key} defaultValue={JSON.stringify(draft.command?.env ?? {}, null, 2)} /></label>
          </>}
          <label className="field">Its messages<select name={'server-channel-' + draft.key} defaultValue={draft.channel ?? 'off'}><option value="off">Never wake the agent</option><option value="wake">Wake an idle agent</option></select></label>
          <button type="button" className="btn" onClick={() => setServers((values) => values.filter((entry) => entry.key !== draft.key))}>Remove server</button>
        </fieldset>)}
        <button type="button" className="btn" onClick={() => { setServers((values) => [...values, { key: nextKey, kind: 'address', name: '', url: '' }]); setNextKey((value) => value + 1); }}>Add MCP server</button>
      </details>
      <details><summary>Permissions</summary><p>Rules as the settings file reads them: a tool name, or a tool name with one specifier in brackets. One per line.</p>
        <label className="field">Allow<textarea name="perm-allow" defaultValue={held?.allow?.join('\n') ?? ''} /></label>
        <label className="field">Deny<textarea name="perm-deny" defaultValue={held?.deny?.join('\n') ?? ''} /></label>
        <label className="field">Ask<textarea name="perm-ask" defaultValue={held?.ask?.join('\n') ?? ''} /></label>
        <label className="field">Permission mode<select name="perm-mode" defaultValue={held?.default_mode ?? ''}><option value="">Not set</option>{MODES.map((mode) => <option key={mode} value={mode}>{mode}</option>)}</select></label>
        <label className="field">Additional directories, absolute<textarea name="perm-dirs" defaultValue={held?.additional_directories?.join('\n') ?? ''} /></label>
      </details>
      <label className="field">Reason for this version<input name="note" required placeholder="What changed and why" /></label>
      <button type="submit" className="btn primary">Record profile</button>
    </fieldset>{error ? <p role="alert">{error}</p> : null}<ChangeStatus change={change} />
  </form>;
}
