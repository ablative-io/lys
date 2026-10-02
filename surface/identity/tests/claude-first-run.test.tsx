import { describe, expect, it } from 'vitest';
import { renderToStaticMarkup } from 'react-dom/server';
import { ProfileFields } from '../src/features/provisioning/ProfileEditor';
import type { Choices, Program } from '../src/features/provisioning/choices';
import catalogue from '../../../docs/harness/catalogue/claude-code.json';

describe('Claude first run', () => {
  it('enables its own default and names the writable temporary folder', () => {
    const program = { ...catalogue, instructions_modes: ['keep'], builds: [{ name: 'Installed', program: '/opt/seat/bin/claude', package: 'claude', from: 'runner' }] } as Program;
    const choices: Choices = { programs: [program], programsMissing: '', machines: [], skills: [], secrets: [] };
    const html = renderToStaticMarkup(<ProfileFields profile={null} choices={choices} firstRun render={(fields, settings, refusal) => <form>{fields}<button disabled={Boolean(refusal)}>Add</button><output>{String((settings.permissions as { default_mode: string }).default_mode)}</output></form>} />);
    expect(html).toContain('<output>workspace-only</output>');
    expect(html).not.toContain('disabled=""');
    expect(html).toContain('Shell commands can also write to the program’s temporary folder.');
  });
});
