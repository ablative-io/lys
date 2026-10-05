import { describe, expect, it } from 'vitest';
import { renderToStaticMarkup } from 'react-dom/server';
import { ProfileFields } from '../src/features/provisioning/ProfileEditor';
import type { Choices, Program } from '../src/features/provisioning/choices';

describe('Claude first run', () => {
  it('enables its own default, shows the mode by its real name and claims only what Lys does', () => {
    const program: Program = { name: 'Claude Code', line: 'Claude Code', models: [{ id: 'default', label: 'Default' }], modes: [{ id: 'workspace-only', meaning: 'Workspace only' }],
      description: { models: { minimum: 1, maximum: null, further_encoding: { kind: 'delimited', separator: ',' } }, permissions: { modes: ['workspace-only'], rule_forms: ['tool_specifier'] }, mcp: { transports: ['http', 'stdio'], working_directory: false, handle_variables: true, channel_policies: ['off'] }, rendering_contract: 'claude-code/template-v1' },
      instructions_modes: ['keep'], builds: [{ name: 'Installed', program: '/opt/seat/bin/claude', package: 'claude', from: 'runner' }] };
    const choices: Choices = { programs: [program], programsMissing: '', machines: [], skills: [], secrets: [] };
    const html = renderToStaticMarkup(<ProfileFields profile={null} choices={choices} firstRun render={(fields, settings, refusal) => <form>{fields}<button disabled={Boolean(refusal)}>Add</button><output>{String((settings.permissions as { default_mode: string }).default_mode)}</output></form>} />);
    expect(html).toContain('<output>workspace-only</output>');
    // The only controls that wait are each table's own add row, until something is typed into it.
    const rest = html.replace(/<button disabled=""[^>]*aria-label="(Add this rule|Add these tools)"[^>]*>.*?<\/button>/g, '');
    expect(rest).not.toContain('disabled=""');
    expect(html).toContain('Kept to its folder');
    expect(html).toContain('Workspace only');
    expect(html).not.toContain('<select name="mode"');
    expect(html).toContain('Lys starts Claude Code without this computer’s own settings, plugins, hooks and connected tools.');
    expect(html).not.toContain('Works in its own folder');
  });
});
