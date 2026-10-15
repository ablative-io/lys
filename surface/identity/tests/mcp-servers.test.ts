/** A connected-tools server is built from a name and what reaches it, and read back in plain words. */
import { describe, expect, it } from 'vitest';
import { kindsFor, others, serverFor, serversFromJson, serverWords } from '../src/features/provisioning/McpServers';

const blank = { name: '', address: '', program: '', args: '' };

describe('connected tools', () => {
  it('builds a server reached at a web address', () => {
    expect(serverFor('address', { ...blank, name: ' notes ', address: ' https://tools.example.org/mcp ' }, [])).toEqual({ server: { name: 'notes', url: 'https://tools.example.org/mcp' } });
  });

  it('builds a server run by a program, with one argument on each line, and with none', () => {
    expect(serverFor('program', { ...blank, name: 'tickets', program: '/usr/local/bin/tickets-mcp', args: '--stdio\n\n --quiet ' }, [])).toEqual({ server: { name: 'tickets', command: { program: '/usr/local/bin/tickets-mcp', args: ['--stdio', '--quiet'] } } });
    expect(serverFor('program', { ...blank, name: 'tickets', program: '/usr/local/bin/tickets-mcp' }, [])).toEqual({ server: { name: 'tickets', command: { program: '/usr/local/bin/tickets-mcp' } } });
  });

  it('says why a server cannot be added, and builds nothing', () => {
    expect(serverFor('address', blank, [])).toEqual({ problem: 'Give it a short name, like notes or tickets.' });
    expect(serverFor('address', { ...blank, name: 'my notes' }, [])).toEqual({ problem: 'The name my notes is letters, digits, _ and - only, with no spaces.' });
    expect(serverFor('address', { ...blank, name: 'notes', address: 'https://x.example' }, ['notes'])).toEqual({ problem: 'This agent already has connected tools named notes.' });
    expect(serverFor('address', { ...blank, name: 'notes' }, [])).toEqual({ problem: 'Type its web address.' });
    expect(serverFor('address', { ...blank, name: 'lys', address: 'https://x.example' }, [])).toEqual({ problem: 'The name lys is taken by Lys’s own tools. Choose another name.' });
    expect(serverFor('address', { ...blank, name: 'Lys', address: 'https://x.example' }, [])).toHaveProperty('problem');
    expect(serverFor('address', { ...blank, name: 'notes', address: 'tools.example.org' }, [])).toHaveProperty('problem');
    expect(serverFor('program', { ...blank, name: 'notes' }, [])).toHaveProperty('problem');
    expect(serverFor('program', { ...blank, name: 'notes', program: 'bad\u0007' }, [])).toHaveProperty('problem');
  });

  it('takes a command by name, as server pages give it, like npx', () => {
    expect(serverFor('program', { ...blank, name: 'notes', program: 'npx', args: '-y\nnotes-mcp' }, [])).toEqual({ server: { name: 'notes', command: { program: 'npx', args: ['-y', 'notes-mcp'] } } });
  });

  it('offers only the ways the program says a server can be reached', () => {
    expect(kindsFor(['stdio', 'http'])).toEqual(['address', 'program']);
    expect(kindsFor(['stdio'])).toEqual(['program']);
    expect(kindsFor([])).toEqual([]);
    expect(kindsFor(undefined)).toEqual([]);
  });

  it('never says the list is the whole of it for a program whose own setup also reaches the run', () => {
    expect(others('Claude Code')).toContain('These are the only others');
    expect(others('Codex')).toContain('Codex also uses whatever connected tools are set up for it');
    expect(others('Codex')).not.toContain('only others');
    expect(others('Another')).toBe('');
  });

  it('says a server in one line of plain words', () => {
    expect(serverWords({ name: 'notes', url: 'https://tools.example.org/mcp' })).toBe('Reached at https://tools.example.org/mcp');
    expect(serverWords({ name: 'tickets', command: { program: '/usr/local/bin/tickets-mcp', args: ['--stdio'] } })).toBe('Runs /usr/local/bin/tickets-mcp --stdio on the agent’s computer');
    expect(serverWords({ name: 'tickets', command: { program: '/usr/local/bin/tickets-mcp' } })).toBe('Runs /usr/local/bin/tickets-mcp on the agent’s computer');
    expect(serverWords({ name: 'empty' })).toBe('No address or program is recorded for it');
  });

  it('reads every server in a pasted mcpServers file, with its arguments, settings and folder', () => {
    const pasted = JSON.stringify({ mcpServers: {
      github: { command: 'npx', args: ['-y', '@modelcontextprotocol/server-github'], env: { GITHUB_PERSONAL_ACCESS_TOKEN: { handle: 'github-token' }, LOG_LEVEL: 'info' } },
      notes: { type: 'http', url: 'https://tools.example.org/mcp' },
      files: { command: '/usr/local/bin/files-mcp', cwd: '/srv/files' },
    } });
    expect(serversFromJson(pasted, ['address', 'program'], [])).toEqual({ servers: [
      { name: 'github', command: { program: 'npx', args: ['-y', '@modelcontextprotocol/server-github'], env: { GITHUB_PERSONAL_ACCESS_TOKEN: { handle: 'github-token' }, LOG_LEVEL: 'info' } } },
      { name: 'notes', url: 'https://tools.example.org/mcp' },
      { name: 'files', command: { program: '/usr/local/bin/files-mcp', cwd: '/srv/files' } },
    ] });
  });

  it('reads the block alone, and one named entry pasted with or without braces', () => {
    const entry = '"notes": { "command": "uvx", "args": ["notes-mcp"] }';
    const want = { servers: [{ name: 'notes', command: { program: 'uvx', args: ['notes-mcp'] } }] };
    expect(serversFromJson('{' + entry + '}', ['program'], [])).toEqual(want);
    expect(serversFromJson(entry, ['program'], [])).toEqual(want);
    expect(serversFromJson(entry + ',', ['program'], [])).toEqual(want);
  });

  it('says plainly why pasted JSON cannot be added, and adds none of it', () => {
    expect(serversFromJson('{ nope', ['program'], [])).toHaveProperty('problem');
    expect(serversFromJson('{ "command": "npx" }', ['program'], [])).toEqual({ problem: 'This is one server without its name. Paste it with its name, like "notes": { ... }.' });
    expect(serversFromJson('{ "mcpServers": {} }', ['program'], [])).toEqual({ problem: 'The JSON names no servers.' });
    expect(serversFromJson('{ "notes": { "command": "npx" } }', ['program'], ['notes'])).toEqual({ problem: 'This agent already has connected tools named notes.' });
    expect(serversFromJson('{ "notes": { "url": "https://x.example/mcp" } }', ['program'], [])).toEqual({ problem: 'notes is reached at a web address, which this program cannot use.' });
    expect(serversFromJson('{ "notes": { "url": "https://x.example/mcp", "headers": { "Authorization": "Bearer x" } } }', ['address'], [])).toHaveProperty('problem');
    expect(serversFromJson('{ "notes": { "command": "npx", "env": { "N": [1] } } }', ['program'], [])).toHaveProperty('problem');
    expect(serversFromJson('{ "a": { "command": "npx" }, "a b": { "command": "npx" } }', ['program'], [])).toHaveProperty('problem');
  });

  it('says a server’s folder and settings in its line, a secret by its name only', () => {
    expect(serverWords({ name: 'gh', command: { program: 'npx', args: ['gh-mcp'], cwd: '/srv', env: { TOKEN: { handle: 'gh' }, LEVEL: 'info' } } })).toBe('Runs npx gh-mcp on the agent’s computer, in /srv, with TOKEN from the secret gh, LEVEL = info');
  });
});
