import { expect, it } from 'vitest';
import { act } from 'react';
import { createRoot } from 'react-dom/client';
import { Provisioning } from './Provisioning';
import { serve, text } from '../../../tests/harness';
import { ADA, SCRIBE, SERVICE, ok } from '../../../tests/fixtures';

it('shows every reviewed seat program field and no native settings', async () => {
  const posted: { path: string; body: unknown }[] = [];
  const spec = { executable: '/opt/seat/host', executable_sha256: 'a'.repeat(64), arguments: ['run', '--config', '{seat_file}'], seat_file: '/opt/seat/seat.json', seat_file_sha256: 'b'.repeat(64), adapter: 'acp/1', model_wire: 'openai-responses' };
  serve({ ...SERVICE,
    ['/agents/' + SCRIBE + '/provisioning']: ok({ agent: SCRIBE, enforced: false, versions: [], profile: {
      version: 1, operation: 'op-review', model_access: [], tools: [], skills: [], mcp_servers: [], instructions: '', note: '', set_by: ADA, set_at: 1,
      harness: { name: 'Seat host', program: spec.executable, package: 'seat 1', description: {
        rendering_contract: 'seat-program/v1', seat_program: spec, models: { minimum: 0, maximum: 0, further_encoding: { kind: 'array' } },
        permissions: { modes: [], rule_forms: [] }, mcp: { transports: [], working_directory: false, handle_variables: false, channel_policies: [] }
      } }
    } }),
    ['POST /agents/' + SCRIBE + '/provisioning/1/review']: ok({ agent: SCRIBE, enforced: false, versions: [], profile: { version: 1, reviewed_by: ADA } })
  }, posted);
  const container = document.createElement('div'); document.body.appendChild(container);
  const root = createRoot(container);
  await act(async () => { root.render(<Provisioning id={SCRIBE} />); });
  try {
    for (const value of [spec.executable, spec.executable_sha256, spec.seat_file, spec.seat_file_sha256, spec.adapter, spec.model_wire, '{seat_file}']) expect(text()).toContain(value);
    expect(document.querySelector('select[name="model"]')).toBeNull();
    expect(document.querySelector('select[name="mode"]')).toBeNull();
    expect(document.querySelector('input[name="config_dir"]')).toBeNull();
    const review = [...document.querySelectorAll('button')].find((button) => button.getAttribute('aria-label') === 'Review these settings');
    expect(review).toBeDefined();
    await act(async () => review?.click());
    expect(posted).toHaveLength(1);
    expect(posted[0].path).toBe('/agents/' + SCRIBE + '/provisioning/1/review');
    expect(text()).toContain('Reviewed');
  } finally { await act(async () => root.unmount()); }
});
