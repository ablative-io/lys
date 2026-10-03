/**
 * The connected tools an agent has: the MCP servers named in its own
 * settings. A Lys-started agent gets these and Lys's own, and none from the
 * person's own setup on the computer (docs/harness/reference/claude-code/
 * CLEAN-START.md), so this list is the whole of it.
 *
 * Each server is one row in plain words. One is added by choosing how it is
 * reached, then giving only what that needs: a web address, or a program on
 * the agent's computer with its arguments. What is typed here is what Lys
 * does not hold a list of; nothing is written as JSON.
 */
import { useState } from 'react';
import type { McpServer } from './Provisioning';

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

function AddServer({ kinds, taken, add, done }: { kinds: ServerKind[]; taken: string[]; add: (server: McpServer) => void; done: () => void }) {
  const [kind, setKind] = useState<ServerKind | null>(kinds.length === 1 ? kinds[0] : null);
  const [given, setGiven] = useState({ name: '', address: '', program: '', args: '' });
  const built = kind ? serverFor(kind, given, taken) : null;
  const server = built && 'server' in built ? built.server : null;
  const set = (field: keyof typeof given) => (event: { target: { value: string } }) => setGiven({ ...given, [field]: event.target.value });
  const noSubmit = (event: { key: string; preventDefault: () => void }) => { if (event.key === 'Enter') event.preventDefault(); };
  return <div className="add-rule" role="group" aria-label="Add connected tools">
    <p><b>How are these tools reached?</b></p>
    {kinds.includes('address') ? <label className="choice-row"><input type="radio" name="server-kind" value="address" checked={kind === 'address'} onChange={() => setKind('address')} /> At a web address</label> : null}
    {kinds.includes('program') ? <label className="choice-row"><input type="radio" name="server-kind" value="program" checked={kind === 'program'} onChange={() => setKind('program')} /> By a program on the agent’s computer</label> : null}
    {kind ? <label className="field">A short name for them<span className="hint">For example: notes</span><input name="server-name" value={given.name} onChange={set('name')} onKeyDown={noSubmit} autoComplete="off" /></label> : null}
    {kind === 'address' ? <label className="field">Web address<span className="hint">For example: https://tools.example.org/mcp</span><input name="server-address" value={given.address} onChange={set('address')} onKeyDown={noSubmit} autoComplete="off" /></label> : null}
    {kind === 'program' ? <>
      <label className="field">The program’s full path<span className="hint">For example: /usr/local/bin/notes-mcp</span><input name="server-program" value={given.program} onChange={set('program')} onKeyDown={noSubmit} autoComplete="off" /></label>
      <label className="field">What it is started with<span className="hint">Optional. One argument on each line.</span><textarea name="server-args" rows={3} value={given.args} onChange={set('args')} /></label>
    </> : null}
    {server ? <p className="rule-row"><b>{server.name}</b> {serverWords(server)}</p> : null}
    <p>
      <button type="button" className="btn primary" disabled={!server} onClick={() => { if (server) { add(server); done(); } }}>Add these tools</button>{' '}
      <button type="button" className="btn" onClick={done}>Cancel</button>
    </p>
    {!kind ? <p className="why-not">Choose how they are reached.</p> : built && 'problem' in built ? <p className="why-not">{built.problem}</p> : null}
  </div>;
}

/** The ways a program's description says a server can be reached, as this screen offers them. */
export function kindsFor(transports: string[] | undefined): ServerKind[] {
  return [...(transports?.includes('http') ? ['address' as const] : []), ...(transports?.includes('stdio') ? ['program' as const] : [])];
}

export function McpServers({ program, transports, value, change }: { program: string; transports: string[] | undefined; value: McpServer[]; change: (next: McpServer[]) => void }) {
  const [adding, setAdding] = useState(false);
  const kinds = kindsFor(transports);
  return <section className="rule-list mcp-servers" aria-label="Connected tools">
    <p><b>Connected tools {program || 'this agent'} can use</b></p>
    <p className="dim">Lys’s own tools are always there. These are the others, and the only others: nothing is taken from anyone’s own setup on the computer.</p>
    {value.length ? <ul>{value.map((server) => <li key={server.name} className="rule-row">
      <b>{server.name}</b> {serverWords(server)}{server.channel === 'wake' ? ', and its messages wake the agent' : ''}{' '}
      <button type="button" className="btn" onClick={() => change(value.filter((one) => one.name !== server.name))}>Remove</button>
    </li>)}</ul> : <p className="dim">None.</p>}
    {kinds.length === 0 ? <p className="dim">{program ? program + ' does not say how connected tools reach it, so none can be added here.' : 'Choose a program first.'}</p>
      : adding ? <AddServer kinds={kinds} taken={value.map((server) => server.name)} add={(server) => change([...value, server])} done={() => setAdding(false)} />
      : <p><button type="button" className="btn" onClick={() => setAdding(true)}>Add connected tools</button></p>}
  </section>;
}
