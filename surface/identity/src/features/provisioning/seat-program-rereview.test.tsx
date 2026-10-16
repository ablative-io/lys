import { expect, it } from 'vitest';
import { act } from 'react';
import { createRoot } from 'react-dom/client';
import { MemoryRouter } from 'react-router';
import { AgentRun } from '../team/AgentRun';
import { serve, text } from '../../../tests/harness';
import { ADA, SCRIBE, SCRIBE_VIEW, SERVICE, ok } from '../../../tests/fixtures';

it('marks a changed seat on the agent row and reviews both found digests with one press', async () => {
  const path = '/agents/' + SCRIBE + '/provisioning';
  const proof = { provisioning_version: 1, contract: 'seat-program/v1', executable: '/opt/seat/host', seat_file: '/opt/seat/seat.json', reviewed_executable_sha256: 'a'.repeat(64), reviewed_seat_file_sha256: 'b'.repeat(64), found_executable_sha256: 'c'.repeat(64), found_seat_file_sha256: 'd'.repeat(64) };
  let profile = { version: 1, operation: 'op-' + '1'.repeat(32), model_access: [], tools: [], skills: [], plugins: [], mcp_servers: [], instructions: '', instructions_mode: 'keep', session: null, permissions: null, note: '', set_by: ADA, set_at: 1, skill_pins: [], reviewed_by: ADA as string | null, reviewed_at: 1 as number | null, self_reviewed: true,
    harness: { name: 'Seat host', program: proof.executable, package: 'seat 1', description: { rendering_contract: 'seat-program/v1', seat_program: { executable: proof.executable, executable_sha256: proof.reviewed_executable_sha256, arguments: ['run', '--config', '{seat_file}'], seat_file: proof.seat_file, seat_file_sha256: proof.reviewed_seat_file_sha256, adapter: 'acp/1' }, models: { minimum: 0, maximum: 0, further_encoding: { kind: 'array' } }, permissions: { modes: [], rule_forms: [] }, mcp: { transports: [], working_directory: false, handle_variables: false, channel_policies: [] } } } };
  const posted: { path: string; body: unknown }[] = [];
  const answer = () => ({ agent: SCRIBE, enforced: false, versions: [], profile });
  serve({ ...SERVICE,
    '/runtime/live': () => ok({ sessions: [{ session: 'run', agent: SCRIBE, machine: 'computer', machine_name: 'Computer', shown: 'running', seat_program: profile.version === 1 ? proof : undefined }] }),
    [path]: () => ok(answer()),
    ['POST ' + path + '/1/seat-program-review']: (body) => {
      const given = body as { operation: string; executable_sha256: string; seat_file_sha256: string };
      profile = { ...profile, version: 2, operation: given.operation, reviewed_by: ADA, reviewed_at: 2, harness: { ...profile.harness, description: { ...profile.harness.description, seat_program: { ...profile.harness.description.seat_program, executable_sha256: given.executable_sha256, seat_file_sha256: given.seat_file_sha256 } } } };
      return ok({ ...answer(), recorded: { operation: given.operation, version: 2 } });
    },
  }, posted);
  const container = document.createElement('div'); document.body.appendChild(container);
  const root = createRoot(container);
  await act(async () => { root.render(<MemoryRouter><AgentRun entry={{ id: SCRIBE, display_name: 'Scribe', kind: 'agent', state: 'active', role: null, person: SCRIBE_VIEW.person }} /></MemoryRouter>); });
  try {
    for (const digest of [proof.reviewed_executable_sha256, proof.reviewed_seat_file_sha256, proof.found_executable_sha256, proof.found_seat_file_sha256]) expect(text()).toContain(digest);
    const button = [...container.querySelectorAll('button')].find((entry) => entry.getAttribute('aria-label') === 'Re-review');
    expect(button).toBeDefined();
    await act(async () => { button?.click(); });
    expect(posted).toHaveLength(1);
    expect(posted[0].path).toBe(path + '/1/seat-program-review');
    expect(posted[0].body).toMatchObject({ executable_sha256: proof.found_executable_sha256, seat_file_sha256: proof.found_seat_file_sha256 });
    expect(profile.reviewed_by).toBe(ADA);
    expect(text()).not.toContain(proof.reviewed_seat_file_sha256);
    expect(container.querySelector('button[aria-label="Re-review"]')).toBeNull();
  } finally { await act(async () => { root.unmount(); }); }
});
