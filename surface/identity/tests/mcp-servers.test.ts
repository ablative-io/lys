/** A connected-tools server is built from a name and what reaches it, and read back in plain words. */
import { describe, expect, it } from 'vitest';
import { kindsFor, serverFor, serverWords } from '../src/features/provisioning/McpServers';

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
    expect(serverFor('address', { ...blank, name: 'my notes' }, [])).toEqual({ problem: 'The name is letters, digits, _ and - only, with no spaces.' });
    expect(serverFor('address', { ...blank, name: 'notes', address: 'https://x.example' }, ['notes'])).toEqual({ problem: 'This agent already has connected tools named notes.' });
    expect(serverFor('address', { ...blank, name: 'notes' }, [])).toEqual({ problem: 'Type its web address.' });
    expect(serverFor('address', { ...blank, name: 'lys', address: 'https://x.example' }, [])).toEqual({ problem: 'The name lys is taken by Lys’s own tools. Choose another name.' });
    expect(serverFor('address', { ...blank, name: 'Lys', address: 'https://x.example' }, [])).toHaveProperty('problem');
    expect(serverFor('address', { ...blank, name: 'notes', address: 'tools.example.org' }, [])).toHaveProperty('problem');
    expect(serverFor('program', { ...blank, name: 'notes' }, [])).toHaveProperty('problem');
    expect(serverFor('program', { ...blank, name: 'notes', program: 'notes-mcp' }, [])).toHaveProperty('problem');
  });

  it('offers only the ways the program says a server can be reached', () => {
    expect(kindsFor(['stdio', 'http'])).toEqual(['address', 'program']);
    expect(kindsFor(['stdio'])).toEqual(['program']);
    expect(kindsFor([])).toEqual([]);
    expect(kindsFor(undefined)).toEqual([]);
  });

  it('says a server in one line of plain words', () => {
    expect(serverWords({ name: 'notes', url: 'https://tools.example.org/mcp' })).toBe('Reached at https://tools.example.org/mcp');
    expect(serverWords({ name: 'tickets', command: { program: '/usr/local/bin/tickets-mcp', args: ['--stdio'] } })).toBe('Runs /usr/local/bin/tickets-mcp --stdio on the agent’s computer');
    expect(serverWords({ name: 'tickets', command: { program: '/usr/local/bin/tickets-mcp' } })).toBe('Runs /usr/local/bin/tickets-mcp on the agent’s computer');
    expect(serverWords({ name: 'empty' })).toBe('No address or program is recorded for it');
  });
});
