import { useState } from 'react';
import type { ReactNode } from 'react';
import type { Choices } from './choices';
import type { ProvisioningProfile } from './Provisioning';
import { StartAgent } from '../runtime/StartAgent';
import type { PeopleView } from '../../generated';

export function ProfileEditor({ id, profile, choices, people, readOnly = false }: {
  id: string; profile: ProvisioningProfile | null; choices: Choices; people: PeopleView; readOnly?: boolean;
}) {
  return <ProfileFields profile={profile} choices={choices} render={(fields, settings, refusal) => <section className="card" aria-label="Start this agent">
    {fields}<StartAgent agent={id} profile={profile} settings={settings} refusal={refusal} canSave={!readOnly} known={{ people, machines: choices.machines }} />
  </section>} />;
}

export function ProfileFields({ profile, choices, strict = false, firstRun = false, render }: {
  profile: ProvisioningProfile | null; choices: Choices; strict?: boolean; firstRun?: boolean;
  render: (fields: ReactNode, settings: Record<string, unknown>, refusal: string) => ReactNode;
}) {
  const available = choices.programs ?? [];
  const workspaceMode = (entry: typeof available[number] | undefined) => entry?.modes.find((mode) => mode.id === 'workspace-only' || mode.id === 'workspace-write')?.id ?? '';
  const initialProgram = firstRun ? available.find((entry) => entry.builds.length > 0 && workspaceMode(entry)) ?? available[0] : available.length === 1 ? available[0] : undefined;
  const [programName, setProgram] = useState(profile?.harness?.name ?? initialProgram?.name ?? '');
  const program = choices.programs?.find((entry) => entry.name === programName);
  const [model, setModel] = useState(profile?.model_access[0] ?? (strict && program?.models.length !== 1 ? '' : program?.models[0]?.id ?? ''));
  const firstMode = (entry: typeof program) => firstRun ? workspaceMode(entry) : strict && entry?.modes.length !== 1 ? '' : entry?.modes[0]?.id ?? '';
  const firstPrompt = (entry: typeof program) => entry?.instructions_modes?.includes('keep') ? 'keep' : entry?.instructions_modes?.includes('append') ? 'append' : '';
  const [mode, setMode] = useState(profile?.permissions?.default_mode ?? firstMode(program));
  const [prompt, setPrompt] = useState(firstRun ? profile?.instructions_mode ?? firstPrompt(program) : strict ? profile?.instructions_mode ?? (program?.instructions_modes?.length === 1 ? program.instructions_modes[0] : '') : profile?.instructions ? profile.instructions_mode ?? 'append' : 'keep');
  const [promptChanged, setPromptChanged] = useState(false);
  const [instructions, setInstructions] = useState(profile?.instructions ?? '');
  const builds = program?.builds ?? [];
  const heldBuild = profile?.harness?.name === programName ? profile.harness : null;
  const heldListed = heldBuild && builds.some((entry) => entry.program === heldBuild.program && entry.package === heldBuild.package);
  const [build, setBuild] = useState(heldBuild ? heldListed ? heldBuild.program + '\n' + heldBuild.package : 'another' : (firstRun ? builds.length > 0 : builds.length === 1) ? builds[0].program + '\n' + builds[0].package : '');
  const [programPath, setProgramPath] = useState(heldBuild?.program ?? '');
  const supported = program?.instructions_modes ?? ['keep', 'append'];
  const selected = build === 'another' && programPath.startsWith('/') && !programPath.includes('\0')
    ? { program: programPath, package: heldBuild && heldBuild.program === programPath ? heldBuild.package : programPath }
    : builds.find((entry) => entry.program + '\n' + entry.package === build);
  let refusal = '';
  if (choices.programs === null) refusal = 'This Lys is too old to list programs. It gets the list when Lys is updated.';
  else if (!program) refusal = choices.programs.length ? 'Choose the program this agent will use.' : 'Lys lists no programs it can start.';
  else if (!selected) refusal = build === 'another' ? 'Enter the program’s full path, starting with /.' : builds.length === 0 ? program.name + ' is not installed on this computer.' + (program.not_found ? ' ' + program.not_found : '') : 'Choose an installed copy of ' + program.name + '.';
  else if (!program.models.some((entry) => entry.id === model)) refusal = 'Choose a model this program lists.';
  else if (!program.modes.some((entry) => entry.id === mode)) refusal = firstRun ? 'This program has no setting that keeps it to its own folder with internet off.' : 'Choose a mode this program lists.';
  else if (!supported.some((entry) => entry === prompt)) refusal = 'This program does not offer that prompt choice.';
  const permissions = { ...profile?.permissions, default_mode: mode };
  const settings: Record<string, unknown> = {
    model_access: profile?.model_access[0] === model ? profile.model_access : [model],
    tools: profile?.tools ?? [], skills: profile?.skills ?? [], mcp_servers: profile?.mcp_servers ?? [],
    instructions: prompt === 'keep' ? !promptChanged && profile ? profile.instructions : '' : instructions.trim(), note: 'Start this agent',
    harness: program && selected ? { name: program.name, description: program.description, program: selected.program, package: selected.package } : null,
    permissions, ...(program?.instructions_modes ? { instructions_mode: !promptChanged && profile ? profile.instructions_mode ?? 'append' : prompt } : {}),
    ...(profile?.runs_on ? { runs_on: profile.runs_on } : {}), ...(profile?.writable ? { writable: profile.writable } : {}),
    ...(profile?.session ? { session: profile.session } : {}),
  };
  const fields = <>
    <label className="field">Program this agent uses<select name="program" value={programName} onChange={(event) => {
      const next = choices.programs?.find((entry) => entry.name === event.target.value);
      setProgram(event.target.value); setModel(strict && next?.models.length !== 1 ? '' : next?.models[0]?.id ?? ''); setMode(firstMode(next)); setPrompt(firstRun ? firstPrompt(next) : strict ? next?.instructions_modes?.length === 1 ? next.instructions_modes[0] : '' : 'keep'); setPromptChanged(true);
      setBuild(next && (firstRun ? next.builds.length > 0 : next.builds.length === 1) ? next.builds[0].program + '\n' + next.builds[0].package : '');
      setProgramPath('');
    }}>
      {!available.some((entry) => entry.name === programName) ? <option value={programName}>{programName || 'Choose a program'}</option> : null}
      {available.map((entry) => <option key={entry.name} value={entry.name}>{entry.name}</option>)}
    </select></label>
    {program ? <label className="field">Installed copy this agent uses<select name="build" value={build} onChange={(event) => setBuild(event.target.value)}><option value="">Choose an installed copy</option>{builds.map((entry) => <option key={entry.program + '\n' + entry.package} value={entry.program + '\n' + entry.package}>{entry.name} — {entry.package}</option>)}<option value="another">Another program…</option></select></label> : null}
    {build === 'another' ? <label className="field">Program path<input name="program-path" value={programPath} onChange={(event) => setProgramPath(event.target.value)} /></label> : null}
    <label className="field">Model this agent uses<select name="model" value={model} onChange={(event) => setModel(event.target.value)}>{!program?.models.some((entry) => entry.id === model) ? <option value={model}>{model || 'Choose a model'}</option> : null}{program?.models.map((entry) => <option key={entry.id} value={entry.id}>{entry.label}</option>)}</select></label>
    {firstRun ? mode === 'workspace-only' ? <p>Works in its own folder; internet tools are off. Shell commands can also write to the program’s temporary folder.</p> : mode === 'workspace-write' ? <p>Works in its own folder; no internet.</p> : profile ? <p>The saved program settings are kept for this request.</p> : null : <label className="field">What this agent may do<select name="mode" value={mode} onChange={(event) => setMode(event.target.value)}>{!program?.modes.some((entry) => entry.id === mode) ? <option value={mode}>{mode || 'Choose a mode'}</option> : null}{program?.modes.map((entry) => <option key={entry.id} value={entry.id}>{entry.meaning}</option>)}</select></label>}
    <label className="field">System prompt this agent uses<select name="prompt" value={prompt} onChange={(event) => { const value = event.target.value; if (value === 'keep' || value === 'append' || value === 'replace') { setPrompt(value); setPromptChanged(true); } }}>{!supported.some((entry) => entry === prompt) ? <option value="">Choose a prompt</option> : null}{supported.map((entry) => <option key={entry} value={entry}>{entry === 'keep' ? "Keep the program’s own prompt" : entry === 'append' ? "Add to the program’s prompt" : "Replace the program’s prompt"}</option>)}</select></label>
    {prompt && prompt !== 'keep' ? <label className="field">{prompt === 'replace' ? 'Prompt this agent uses instead' : 'Words added to this agent’s prompt'}<span className="hint">Optional.</span><textarea name="instructions" rows={4} value={instructions} onChange={(event) => setInstructions(event.target.value)} /></label> : null}
    <p>Lys makes this agent’s own folder when it starts.</p>
  </>;
  return render(fields, settings, refusal);
}
