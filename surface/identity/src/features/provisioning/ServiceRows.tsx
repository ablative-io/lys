/** Connected services: each is written into the start's MCP settings file; a program is started by the agent's program, an address is connected to. */
import { useState } from 'react';
import type { McpServer, Setting } from './Provisioning';

type Env = { key: number; name: string; value: string; secret: boolean; kept?: Setting };
export type ServiceDraft = { key: number; name: string; kind: 'address' | 'command'; url: string; program: string; args: string; cwd: string; env: Env[]; wake: boolean };

export function draftOf(server: McpServer, key: number): ServiceDraft {
  const env = Object.entries(server.command?.env ?? {}).map(([name, value], index) => typeof value === 'object'
    ? { key: index, name, value: value.handle, secret: true }
    : { key: index, name, value: String(value), secret: false, kept: value });
  return { key, name: server.name, kind: server.command ? 'command' : 'address', url: server.url ?? '', program: server.command?.program ?? '',
    args: (server.command?.args ?? []).join('\n'), cwd: server.command?.cwd ?? '', env, wake: server.channel === 'wake' };
}

export function serverOf(draft: ServiceDraft): McpServer {
  const name = draft.name.trim();
  if (!name) throw new Error('Give each connected service a name.');
  const channel = draft.wake ? 'wake' : 'off';
  if (draft.kind === 'address') {
    const parsed = new URL(draft.url.trim());
    if (!['http:', 'https:'].includes(parsed.protocol) || parsed.username || parsed.password) throw new Error(name + ': the address must start with http:// or https:// and carry no username or password.');
    return { name, url: draft.url.trim(), channel };
  }
  if (!draft.program.trim()) throw new Error(name + ': say which program starts this service.');
  const env: Record<string, Setting> = {};
  for (const row of draft.env) {
    if (!row.name.trim()) continue;
    env[row.name.trim()] = row.secret ? { handle: row.value } : row.kept !== undefined && String(row.kept) === row.value ? row.kept : row.value;
  }
  const args = draft.args === '' ? [] : draft.args.split('\n');
  return { name, command: { program: draft.program.trim(), args, ...(draft.cwd.trim() ? { cwd: draft.cwd.trim() } : {}), env }, channel };
}

export function ServiceRows({ drafts, setDrafts, secrets }: { drafts: ServiceDraft[]; setDrafts: (next: (values: ServiceDraft[]) => ServiceDraft[]) => void; secrets: string[] | null }) {
  const [next, setNext] = useState(drafts.length);
  const edit = (key: number, change: Partial<ServiceDraft>) => setDrafts((values) => values.map((draft) => draft.key === key ? { ...draft, ...change } : draft));
  const editEnv = (draft: ServiceDraft, key: number, change: Partial<Env>) => edit(draft.key, { env: draft.env.map((row) => row.key === key ? { ...row, ...change } : row) });
  return <>
    {drafts.map((draft) => <fieldset key={draft.key} className="card" aria-label={'Connected service ' + (draft.name || 'unnamed')}>
      <label className="field">Name<span className="hint">What the agent calls this service.</span><input value={draft.name} onChange={(event) => edit(draft.key, { name: event.target.value })} required /></label>
      <label className="field">How it is reached<select value={draft.kind} onChange={(event) => edit(draft.key, { kind: event.target.value === 'command' ? 'command' : 'address' })}>
        <option value="command">A program on the agent's computer</option><option value="address">A web address</option></select></label>
      {draft.kind === 'address'
        ? <label className="field">Address<span className="hint">The agent's program connects to this address each time the agent starts.</span><input type="url" value={draft.url} onChange={(event) => edit(draft.key, { url: event.target.value })} required /></label>
        : <>
          <label className="field">Program<span className="hint">The agent's program starts this on the agent's computer each time the agent starts, and talks to it while it runs.</span><input value={draft.program} onChange={(event) => edit(draft.key, { program: event.target.value })} required /></label>
          <label className="field">Given to it when started<span className="hint">One item per line, passed exactly as written, in this order.</span><textarea rows={3} value={draft.args} onChange={(event) => edit(draft.key, { args: event.target.value })} /></label>
          <label className="field">Folder it starts in<span className="hint">Leave empty to start it in the agent's own folder.</span><input value={draft.cwd} onChange={(event) => edit(draft.key, { cwd: event.target.value })} /></label>
          {draft.env.map((row) => <div key={row.key} className="field">Setting it needs
            <input aria-label="Setting name" value={row.name} placeholder="Name" onChange={(event) => editEnv(draft, row.key, { name: event.target.value })} />
            {row.secret
              ? <select aria-label="Secret" value={row.value} onChange={(event) => editEnv(draft, row.key, { value: event.target.value })}><option value="">Choose a secret</option>{(secrets ?? []).map((name) => <option key={name} value={name}>{name}</option>)}</select>
              : <input aria-label="Setting value" value={row.value} placeholder="Value" onChange={(event) => editEnv(draft, row.key, { value: event.target.value })} />}
            <label><input type="checkbox" checked={row.secret} onChange={(event) => editEnv(draft, row.key, { secret: event.target.checked, value: '' })} /> A secret: the agent is given a handle to it, never the secret itself</label>
            <button type="button" className="btn" onClick={() => edit(draft.key, { env: draft.env.filter((entry) => entry.key !== row.key) })}>Remove setting</button>
          </div>)}
          <button type="button" className="btn" onClick={() => edit(draft.key, { env: [...draft.env, { key: draft.env.reduce((most, row) => Math.max(most, row.key + 1), 0), name: '', value: '', secret: false }] })}>Add a setting</button>
        </>}
      <label><input type="checkbox" checked={draft.wake} onChange={(event) => edit(draft.key, { wake: event.target.checked })} /> Let this service's messages wake the agent when it is idle (passed to Claude Code as --channels)</label>
      <button type="button" className="btn" onClick={() => setDrafts((values) => values.filter((entry) => entry.key !== draft.key))}>Remove this service</button>
    </fieldset>)}
    <button type="button" className="btn" onClick={() => { setDrafts((values) => [...values, { key: next, name: '', kind: 'command', url: '', program: '', args: '', cwd: '', env: [], wake: false }]); setNext((value) => value + 1); }}>Add a connected service</button>
  </>;
}
