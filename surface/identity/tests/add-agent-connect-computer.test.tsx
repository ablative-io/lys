import { describe, expect, it } from 'vitest';
import { $, type } from './harness';
import { ADA, OWN, ok, refused } from './fixtures';
import { addAndRun, CONNECT_COMPUTER, newAddAndRun } from '../src/features/people/add-and-run';
import { newAgentRequest } from '../src/features/people/add-agent-request';
import { agent, description, existingComputer, open, service, submit } from './add-and-run-fixture';

function personal(computers = [] as ReturnType<typeof existingComputer>[]) {
  const server = service(undefined, 'running', computers);
  server.routes['/directory/people'] = refused(403, 'NotAdmitted', 'Only an administrator reads the directory.');
  server.routes['/people'] = ok(OWN);
  server.routes['/network'] = ok({ machines: computers, reports_served: true, host_name: 'Identity server' });
  return server;
}

describe('Adding an agent before connecting a personal computer', () => {
  it.each(['none', 'someone else', 'offline'] as const)('offers Add agent only when the connected computer is %s', async (caseName) => {
    const computer = existingComputer();
    if (caseName === 'someone else') computer.named_by = 'person-' + 'b'.repeat(32);
    const server = personal(caseName === 'none' ? [] : [computer]);
    if (caseName === 'offline') server.routes['/network/machines/' + computer.id + '/runner'] = ok({ machine: computer.id, runner: { kind: 'lys' }, answers: false });
    const { posted } = await open(server.routes);
    expect($('[name="computer_name"]')).toBeNull();
    expect(document.body.textContent).not.toContain('Identity server');
    expect(document.body.textContent).toContain(CONNECT_COMPUTER);
    expect([...document.querySelectorAll<HTMLAnchorElement>('a')].find((link) => link.textContent === CONNECT_COMPUTER)?.getAttribute('href')).toBe('#/network');
    await type($('[name="display_name"]'), 'Clover');
    const button = $('form[aria-label="Add an agent"] button[type="submit"]') as HTMLButtonElement;
    expect(button.getAttribute('aria-label')).toBe('Add agent');
    expect(button.disabled).toBe(false);
    await submit();
    expect(posted.filter((entry) => entry.path.startsWith('/network/'))).toEqual([]);
    expect(posted.some((entry) => entry.path === '/agents')).toBe(true);
    expect(location.hash).toBe('#/file/' + agent);
  });

  it('offers an answering computer owned by the signed-in person without adding another computer', async () => {
    const server = personal([existingComputer()]);
    await open(server.routes);
    await type($('[name="display_name"]'), 'Clover');
    expect($('[name="computer_name"]')).toBeNull();
    expect($('form button[type="submit"]')?.getAttribute('aria-label')).toBe('Add Clover and run it on Front desk');
    expect(document.body.textContent).not.toContain(CONNECT_COMPUTER);
  });

  it('refuses a new-computer request before registering an agent and guards old saved requests', async () => {
    const server = personal();
    const { posted } = await open(server.routes);
    const registration = newAgentRequest('Clover', ADA, '');
    const settings = { harness: { name: 'Care', program: '/opt/care', package: 'care', description }, model_access: ['care'], permissions: { default_mode: 'default' }, tools: [], skills: [], mcp_servers: [], instructions: '', note: '' };
    expect(() => newAddAndRun(registration, settings, 'Identity server', ADA)).toThrow(CONNECT_COMPUTER);
    const saved = newAddAndRun(registration, settings, 'Identity server', ADA, undefined, true);
    const kept: unknown[] = [];
    await expect(addAndRun(saved, { provider: 'test', subject: 'ada' }, (next) => { kept.push(next); })).rejects.toMatchObject({ problem: { refusal: { refusal: 'ComputerConnectionRequired' } } });
    expect(kept).toEqual([]);
    expect(posted).toEqual([]);
  });
});
