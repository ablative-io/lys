/**
 * The connected tools an agent is given: the MCP servers named in its own
 * settings. A Lys-started Claude Code gets these and Lys's own, and none from
 * the person's own setup on the computer (docs/harness/reference/claude-code/
 * CLEAN-START.md); a Codex run also reads the login's own Codex setup, so
 * for Codex this list is not the whole of it, and the screen says so.
 *
 * They are one table: the name, how it is reached in plain words, and Remove.
 * A server arrives as the JSON its makers publish (an `mcpServers` block, or
 * one entry of it), so the first way in is to paste that JSON as it was given:
 * every server in it is read, shown in plain words, and added with one press.
 * The last row also adds one by hand: a web address, or a program on the
 * agent's computer with what it is started with and its settings.
 */
import { useState } from 'react';
import type { McpServer, Setting } from './Provisioning';
import { Act } from '../../shell/Act';
import { TableScroll } from '../../shell/Listing';

export type ServerKind = 'address' | 'program';
export type BuiltServer = { server: McpServer } | { problem: string };
export type ReadServers = { servers: McpServer[] } | { problem: string };

const plain = (text: string) => ![...text].some((c) => c < ' ' || c === '\u007f');
const isObject = (value: unknown): value is Record<string, unknown> => typeof value === 'object' && value !== null && !Array.isArray(value);
const nameProblem = (name: string, taken: string[]): string | null => {
  if (!name) return 'Give it a short name, like notes or tickets.';
  if (!/^[A-Za-z0-9_-]+$/.test(name)) return 'The name ' + name + ' is letters, digits, _ and - only, with no spaces.';
  if (name.toLowerCase() === 'lys') return 'The name lys is taken by Lys’s own tools. Choose another name.';
  if (taken.includes(name)) return 'This agent already has connected tools named ' + name + '.';
  return null;
};

/** One entry of an `mcpServers` block, as Claude Code, Codex and most servers' own pages write it, as the server Lys records. */
function entryServer(name: string, entry: unknown, kinds: ServerKind[]): BuiltServer {
  if (!isObject(entry)) return { problem: name + ' is not written as an object with a command or a url.' };
  const url = typeof entry.url === 'string' ? entry.url.trim() : typeof entry.serverUrl === 'string' ? entry.serverUrl.trim() : '';
  if (url) {
    if (!kinds.includes('address')) return { problem: name + ' is reached at a web address, which this program cannot use.' };
    if (!/^https?:\/\/[^\s/]+/.test(url) || !plain(url)) return { problem: 'The url of ' + name + ' does not start with https:// or http://.' };
    if (isObject(entry.headers) && Object.keys(entry.headers).length) return { problem: name + ' sends headers (' + Object.keys(entry.headers).join(', ') + '), which Lys does not pass to a web address yet. Give its key as a setting of a program server instead, or leave it out.' };
    return { server: { name, url } };
  }
  if (typeof entry.command !== 'string' || !entry.command.trim()) return { problem: name + ' names neither a command nor a url.' };
  if (!kinds.includes('program')) return { problem: name + ' is started as a program, which this program cannot use.' };
  const program = entry.command.trim();
  if (!plain(program)) return { problem: 'The command of ' + name + ' holds a character a program cannot be started with.' };
  if (entry.args !== undefined && !(Array.isArray(entry.args) && entry.args.every((arg) => typeof arg === 'string'))) return { problem: 'The args of ' + name + ' are not a list of words.' };
  const args = (entry.args as string[] | undefined) ?? [];
  let env: Record<string, Setting> | undefined;
  if (entry.env !== undefined) {
    if (!isObject(entry.env)) return { problem: 'The env of ' + name + ' is not a list of names and values.' };
    env = {};
    for (const [variable, value] of Object.entries(entry.env)) {
      if (!/^[A-Za-z_][A-Za-z0-9_]*$/.test(variable)) return { problem: 'The setting ' + variable + ' of ' + name + ' is not a variable name.' };
      if (typeof value === 'string' || typeof value === 'boolean' || typeof value === 'number' && Number.isInteger(value)) env[variable] = value;
      else if (isObject(value) && typeof value.handle === 'string' && Object.keys(value).length === 1) env[variable] = { handle: value.handle };
      else return { problem: 'The setting ' + variable + ' of ' + name + ' is not text, a whole number or true/false.' };
    }
  }
  const cwd = typeof entry.cwd === 'string' && entry.cwd.trim() ? entry.cwd.trim() : undefined;
  return { server: { name, command: { program, ...(args.length ? { args } : {}), ...(cwd ? { cwd } : {}), ...(env && Object.keys(env).length ? { env } : {}) } } };
}

/** Every server in pasted JSON: a whole `{ "mcpServers": { ... } }` file, the block inside it, or one `"name": { ... }` entry. */
export function serversFromJson(text: string, kinds: ServerKind[], taken: string[]): ReadServers {
  const trimmed = text.trim();
  if (!trimmed) return { problem: 'Paste the JSON the server’s page gives.' };
  let parsed: unknown;
  try { parsed = JSON.parse(trimmed.startsWith('"') ? '{' + trimmed.replace(/,\s*$/, '') + '}' : trimmed); }
  catch (error) { return { problem: 'This is not JSON Lys can read: ' + (error instanceof Error ? error.message : String(error)) + '.' }; }
  if (!isObject(parsed)) return { problem: 'Paste the JSON object the server’s page gives, starting with {.' };
  const block = isObject(parsed.mcpServers) ? parsed.mcpServers : isObject(parsed.servers) ? parsed.servers : isObject(parsed.mcp_servers) ? parsed.mcp_servers : parsed;
  const looksLikeOne = typeof block.command === 'string' || typeof block.url === 'string';
  if (looksLikeOne) return { problem: 'This is one server without its name. Paste it with its name, like "notes": { ... }.' };
  const names = Object.keys(block);
  if (!names.length) return { problem: 'The JSON names no servers.' };
  const servers: McpServer[] = [];
  for (const name of names) {
    const problem = nameProblem(name, [...taken, ...servers.map((server) => server.name)]);
    if (problem) return { problem };
    const built = entryServer(name, block[name], kinds);
    if ('problem' in built) return built;
    servers.push(built.server);
  }
  return { servers };
}

/** The server for a name and what reaches it, or the sentence that says why it cannot be added. */
export function serverFor(kind: ServerKind, given: { name: string; address: string; program: string; args: string }, taken: string[]): BuiltServer {
  const name = given.name.trim();
  const problem = nameProblem(name, taken);
  if (problem) return { problem };
  if (kind === 'address') {
    const url = given.address.trim();
    if (!url) return { problem: 'Type its web address.' };
    if (!plain(url) || !/^https?:\/\/[^\s/]+/.test(url)) return { problem: 'A web address starts with https:// or http:// and has no spaces.' };
    return { server: { name, url } };
  }
  const program = given.program.trim();
  if (!program) return { problem: 'Type the command that starts it, like npx or /usr/local/bin/notes-mcp.' };
  if (!plain(program)) return { problem: 'The command holds a character a program cannot be started with.' };
  const args = given.args.split('\n').map((line) => line.trim()).filter(Boolean);
  if (args.some((line) => line.includes('\0'))) return { problem: 'An argument holds a character that cannot be passed to a program.' };
  return { server: { name, command: args.length ? { program, args } : { program } } };
}

/** A server in one line of plain words. */
export function serverWords(server: McpServer): string {
  if (server.command) {
    const settings = Object.entries(server.command.env ?? {}).map(([variable, value]) => variable + (typeof value === 'object' ? ' from the secret ' + value.handle : ' = ' + String(value)));
    return 'Runs ' + server.command.program + (server.command.args?.length ? ' ' + server.command.args.join(' ') : '') + ' on the agent’s computer'
      + (server.command.cwd ? ', in ' + server.command.cwd : '') + (settings.length ? ', with ' + settings.join(', ') : '');
  }
  return server.url ? 'Reached at ' + server.url : 'No address or program is recorded for it';
}

/** The table's last row: a name, how the tools are reached, what that needs, and Add. */
function AddServer({ kinds, taken, add }: { kinds: ServerKind[]; taken: string[]; add: (server: McpServer) => void }) {
  const [kind, setKind] = useState<ServerKind | ''>(kinds.length === 1 ? kinds[0] : '');
  const empty = { name: '', address: '', program: '', args: '' };
  const [given, setGiven] = useState(empty);
  const built = kind ? serverFor(kind, given, taken) : null;
  const server = built && 'server' in built ? built.server : null;
  const touched = Boolean(given.name || given.address || given.program || given.args);
  const set = (field: keyof typeof given) => (event: { target: { value: string } }) => setGiven({ ...given, [field]: event.target.value });
  const noSubmit = (event: { key: string; preventDefault: () => void }) => { if (event.key === 'Enter') event.preventDefault(); };
  return <tr className="add-rule" role="group" aria-label="Add connected tools">
    <td><label className="field">Name<input name="server-name" aria-label="A short name for them" placeholder="notes" value={given.name} onChange={set('name')} onKeyDown={noSubmit} autoComplete="off" spellCheck={false} /><span className="hint">A short name, like notes.</span></label></td>
    <td>
      <label className="field">How they are reached<select name="server-kind" aria-label="How these tools are reached" value={kind} onChange={(event) => setKind(event.target.value as ServerKind | '')}>
        {kinds.length === 1 ? null : <option value="">Choose how they are reached</option>}
        {kinds.includes('address') ? <option value="address">At a web address</option> : null}
        {kinds.includes('program') ? <option value="program">By a program on the agent’s computer</option> : null}
      </select></label>
      {kind === 'address' ? <label className="field">Web address<input name="server-address" type="url" aria-label="Web address" placeholder="https://tools.example.org/mcp" value={given.address} onChange={set('address')} onKeyDown={noSubmit} autoComplete="off" spellCheck={false} /></label> : null}
      {kind === 'program' ? <>
        <label className="field">The command that starts it<input name="server-program" aria-label="The command that starts it" placeholder="npx" value={given.program} onChange={set('program')} onKeyDown={noSubmit} autoComplete="off" spellCheck={false} /></label>
        <label className="field">What it is started with <span className="hint">Optional. One on each line; leave empty if there are none.</span><textarea name="server-args" aria-label="What it is started with, one word on each line" rows={3} value={given.args} onChange={set('args')} spellCheck={false} /></label>
      </> : null}
      {server ? <p className="rule-row"><b>{server.name}</b> {serverWords(server)}</p> : null}
      {touched && built && 'problem' in built ? <p className="why-not">{built.problem}</p> : null}
    </td>
    <td><Act symbol="add" name="Add these tools" word="Add" tone="primary" disabled={!server} onClick={() => { if (server) { add(server); setGiven(empty); } }} />
      {!kind && touched ? <p className="why-not">Choose how they are reached.</p> : null}</td>
  </tr>;
}

/** The first way in: paste the JSON a server's page gives, see every server it names in plain words, add them all. */
function PasteServers({ kinds, taken, add }: { kinds: ServerKind[]; taken: string[]; add: (servers: McpServer[]) => void }) {
  const [text, setText] = useState('');
  const read = text.trim() ? serversFromJson(text, kinds, taken) : null;
  const servers = read && 'servers' in read ? read.servers : [];
  return <div className="mcp-paste" role="group" aria-label="Paste connected tools as JSON">
    <label className="field">Paste the JSON from the server’s page
      <span className="hint">As it is given: a whole <code>mcpServers</code> block, or one <code>"name": {'{ … }'}</code> entry. Keys go in as secrets, not here.</span>
      <textarea name="server-json" aria-label="The JSON from the server’s page" rows={6} value={text} onChange={(event) => setText(event.target.value)} spellCheck={false} placeholder={'{\n  "mcpServers": {\n    "notes": { "command": "npx", "args": ["-y", "notes-mcp"] }\n  }\n}'} />
    </label>
    {servers.length ? <ul className="mcp-paste-read">{servers.map((server) => <li key={server.name}><b>{server.name}</b> {serverWords(server)}</li>)}</ul> : null}
    {read && 'problem' in read ? <p className="why-not" role="alert">{read.problem}</p> : null}
    <Act symbol="add" name={servers.length === 1 ? 'Add ' + servers[0].name : servers.length ? 'Add these ' + String(servers.length) + ' servers' : 'Add the pasted tools'} word={servers.length > 1 ? 'Add all ' + String(servers.length) : 'Add'} tone="primary" disabled={!servers.length} onClick={() => { add(servers); setText(''); }} />
  </div>;
}

/**
 * What else reaches the run beside this list, said only for a program it is
 * known of (docs/harness/reference/claude-code/CLEAN-START.md rows 4, 7a, 14):
 * Claude Code is started without the person's own setup; Codex is not.
 */
export function others(program: string): string {
  if (program === 'Claude Code') return 'Lys’s own tools are there too. These are the only others: Lys starts Claude Code without the connected tools of anyone’s own setup on its computer.';
  if (program === 'Codex') return 'Lys’s own tools are there too. Codex also uses whatever connected tools are set up for it on the agent’s computer; Lys does not list those here.';
  return '';
}

/** The ways a program's description says a server can be reached, as this screen offers them. */
export function kindsFor(transports: string[] | undefined): ServerKind[] {
  return [...(transports?.includes('http') ? ['address' as const] : []), ...(transports?.includes('stdio') ? ['program' as const] : [])];
}

export function McpServers({ program, transports, value, change }: { program: string; transports: string[] | undefined; value: McpServer[]; change: (next: McpServer[]) => void }) {
  const kinds = kindsFor(transports);
  return <section className="mcp-servers wide" aria-label="Connected tools">
    <TableScroll label="Connected tools"><table className="usage-table">
      <colgroup><col style={{ width: '22%' }} /><col style={{ width: '64%' }} /><col style={{ width: '14%' }} /></colgroup>
      <thead><tr><th>Connected tools {program || 'this agent'} can use</th><th>How they are reached</th><th>Change</th></tr></thead>
      <tbody>
        {value.map((server) => <tr key={server.name} className="rule-row">
          <td><b>{server.name}</b></td>
          <td>{serverWords(server)}{server.channel === 'wake' ? ', and its messages wake the agent' : ''}</td>
          <td><Act symbol="remove" name={'Remove ' + server.name} onClick={() => change(value.filter((one) => one.name !== server.name))} /></td>
        </tr>)}
        {value.length ? null : <tr><td colSpan={3} className="dim">None.</td></tr>}
      </tbody>
      {kinds.length ? <tfoot><AddServer kinds={kinds} taken={value.map((server) => server.name)} add={(server) => change([...value, server])} /></tfoot> : null}
    </table></TableScroll>
    {kinds.length ? <PasteServers kinds={kinds} taken={value.map((server) => server.name)} add={(servers) => change([...value, ...servers])} /> : null}
    {kinds.length === 0 ? <p className="dim">{program ? program + ' does not say how connected tools reach it, so none can be added here.' : 'Choose a program first.'}</p> : null}
    {others(program) ? <p className="dim">{others(program)}</p> : null}
  </section>;
}
