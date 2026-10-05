/**
 * The connected tools an agent is given: the MCP servers named in its own
 * settings. A Lys-started Claude Code gets these and Lys's own, and none from
 * the person's own setup on the computer (docs/harness/reference/claude-code/
 * CLEAN-START.md); a Codex run also reads the login's own Codex setup, so
 * for Codex this list is not the whole of it, and the screen says so.
 *
 * They are one table: the name, how it is reached in plain words, and Remove.
 * Its last row adds one by choosing how it is reached, then giving only what
 * that needs: a web address, or a program on the agent's computer with its
 * arguments. What is typed here is what Lys does not hold a list of; nothing
 * is written as JSON.
 */
import { useState } from 'react';
import type { McpServer } from './Provisioning';
import { Act } from '../../shell/Act';

export type ServerKind = 'address' | 'program';
export type BuiltServer = { server: McpServer } | { problem: string };

const plain = (text: string) => ![...text].some((c) => c < ' ' || c === '\u007f');

/** The server for a name and what reaches it, or the sentence that says why it cannot be added. */
export function serverFor(kind: ServerKind, given: { name: string; address: string; program: string; args: string }, taken: string[]): BuiltServer {
  const name = given.name.trim();
  if (!name) return { problem: 'Give it a short name, like notes or tickets.' };
  if (!/^[A-Za-z0-9_-]+$/.test(name)) return { problem: 'The name is letters, digits, _ and - only, with no spaces.' };
  if (name.toLowerCase() === 'lys') return { problem: 'The name lys is taken by Lys’s own tools. Choose another name.' };
  if (taken.includes(name)) return { problem: 'This agent already has connected tools named ' + name + '.' };
  if (kind === 'address') {
    const url = given.address.trim();
    if (!url) return { problem: 'Type its web address.' };
    if (!plain(url) || !/^https?:\/\/[^\s/]+/.test(url)) return { problem: 'A web address starts with https:// or http:// and has no spaces.' };
    return { server: { name, url } };
  }
  const program = given.program.trim();
  if (!program) return { problem: 'Type the program’s full path on the agent’s computer.' };
  if (!program.startsWith('/') || !plain(program)) return { problem: 'The program’s path starts with / and is the full path on the agent’s computer.' };
  const args = given.args.split('\n').map((line) => line.trim()).filter(Boolean);
  if (args.some((line) => line.includes('\0'))) return { problem: 'An argument holds a character that cannot be passed to a program.' };
  return { server: { name, command: args.length ? { program, args } : { program } } };
}

/** A server in one line of plain words. */
export function serverWords(server: McpServer): string {
  if (server.command) return 'Runs ' + server.command.program + (server.command.args?.length ? ' ' + server.command.args.join(' ') : '') + ' on the agent’s computer';
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
    <td><input name="server-name" aria-label="A short name for them" placeholder="A short name, like notes" value={given.name} onChange={set('name')} onKeyDown={noSubmit} autoComplete="off" /></td>
    <td>
      <select name="server-kind" aria-label="How these tools are reached" value={kind} onChange={(event) => setKind(event.target.value as ServerKind | '')}>
        {kinds.length === 1 ? null : <option value="">Choose how they are reached</option>}
        {kinds.includes('address') ? <option value="address">At a web address</option> : null}
        {kinds.includes('program') ? <option value="program">By a program on the agent’s computer</option> : null}
      </select>
      {kind === 'address' ? <input name="server-address" aria-label="Web address" placeholder="https://tools.example.org/mcp" value={given.address} onChange={set('address')} onKeyDown={noSubmit} autoComplete="off" /> : null}
      {kind === 'program' ? <>
        <input name="server-program" aria-label="The program’s full path" placeholder="/usr/local/bin/notes-mcp" value={given.program} onChange={set('program')} onKeyDown={noSubmit} autoComplete="off" />
        <textarea name="server-args" aria-label="What it is started with, one word on each line" placeholder="What it is started with, one on each line. Leave empty if there are none." rows={3} value={given.args} onChange={set('args')} />
      </> : null}
      {server ? <p className="rule-row"><b>{server.name}</b> {serverWords(server)}</p> : null}
      {touched && built && 'problem' in built ? <p className="why-not">{built.problem}</p> : null}
    </td>
    <td><Act symbol="add" name="Add these tools" word="Add" tone="primary" disabled={!server} onClick={() => { if (server) { add(server); setGiven(empty); } }} /></td>
  </tr>;
}

/**
 * What else reaches the run beside this list, said only for a program it is
 * known of (docs/harness/reference/claude-code/CLEAN-START.md rows 4, 7a, 14):
 * Claude Code is started without the person's own setup; Codex is not.
 */
export function others(program: string): string {
  if (program === 'Claude Code') return 'Lys’s own tools are there too. These are the only others: Lys starts Claude Code without the connected tools of anyone’s own setup on this computer.';
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
    <table className="usage-table">
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
    </table>
    {kinds.length === 0 ? <p className="dim">{program ? program + ' does not say how connected tools reach it, so none can be added here.' : 'Choose a program first.'}</p> : null}
    {others(program) ? <p className="dim">{others(program)}</p> : null}
  </section>;
}
