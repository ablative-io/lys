import { useState } from 'react';
import type { Choices } from './choices';
import type { ProvisioningProfile } from './Provisioning';
import { StartAgent } from '../runtime/StartAgent';

export function ProfileEditor({ id, profile, choices, readOnly = false }: {
  id: string; profile: ProvisioningProfile | null; choices: Choices; readOnly?: boolean;
}) {
  const available = choices.programs?.filter((entry) => entry.builds.length > 0) ?? [];
  const [programName, setProgram] = useState(profile?.harness?.name ?? (available.length === 1 ? available[0].name : choices.programs?.length === 1 ? choices.programs[0].name : ''));
  const program = choices.programs?.find((entry) => entry.name === programName);
  const [model, setModel] = useState(profile?.model_access[0] ?? program?.models[0]?.id ?? '');
  const [mode, setMode] = useState(profile?.permissions?.default_mode ?? program?.modes[0]?.id ?? '');
  const [prompt, setPrompt] = useState(profile?.instructions ? profile.instructions_mode ?? 'append' : 'keep');
  const [promptChanged, setPromptChanged] = useState(false);
  const [instructions, setInstructions] = useState(profile?.instructions ?? '');
  const builds = program?.builds ?? [];
  const heldBuild = profile?.harness?.name === programName ? profile.harness : null;
  const [build, setBuild] = useState(heldBuild ? heldBuild.program + '\n' + heldBuild.package : builds.length === 1 ? builds[0].program + '\n' + builds[0].package : '');
  const supported = program?.instructions_modes ?? ['keep', 'append'];
  const selected = builds.find((entry) => entry.program + '\n' + entry.package === build) ?? heldBuild;
  let refusal = '';
  if (choices.programs === null) refusal = 'This Lys is too old to list programs. It gets the list when Lys is updated.';
  else if (!program) refusal = choices.programs.length ? 'Choose the program this agent will use.' : 'ProgramUnavailable: Lys lists no startable programs.';
  else if (!selected) refusal = 'ProgramBuildUnavailable: Lys has no selected installed copy of this program.';
  else if (!program.models.some((entry) => entry.id === model)) refusal = 'ModelUnavailable: choose a model this program lists.';
  else if (!program.modes.some((entry) => entry.id === mode)) refusal = 'ModeUnavailable: choose a mode this program lists.';
  else if (!supported.some((entry) => entry === prompt)) refusal = 'PromptReplacementUnavailable: this program does not list that prompt choice.';
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
  return <section className="card" aria-label="Start this agent">
    <label className="field">Program this agent uses<select name="program" value={programName} onChange={(event) => {
      const next = choices.programs?.find((entry) => entry.name === event.target.value);
      setProgram(event.target.value); setModel(next?.models[0]?.id ?? ''); setMode(next?.modes[0]?.id ?? ''); setPrompt('keep'); setPromptChanged(true);
      setBuild(next?.builds.length === 1 ? next.builds[0].program + '\n' + next.builds[0].package : '');
    }}>
      {!available.some((entry) => entry.name === programName) ? <option value={programName}>{programName || 'Choose a program'}</option> : null}
      {available.map((entry) => <option key={entry.name} value={entry.name}>{entry.name}</option>)}
    </select></label>
    {builds.length > 1 ? <label className="field">Installed copy this agent uses<select name="build" value={build} onChange={(event) => setBuild(event.target.value)}><option value="">Choose an installed copy</option>{builds.map((entry) => <option key={entry.program + '\n' + entry.package} value={entry.program + '\n' + entry.package}>{entry.name} — {entry.package}</option>)}</select></label> : null}
    <label className="field">Model this agent uses<select name="model" value={model} onChange={(event) => setModel(event.target.value)}>{!program?.models.some((entry) => entry.id === model) ? <option value={model}>{model || 'Choose a model'}</option> : null}{program?.models.map((entry) => <option key={entry.id} value={entry.id}>{entry.label}</option>)}</select></label>
    <label className="field">What this agent may do<select name="mode" value={mode} onChange={(event) => setMode(event.target.value)}>{!program?.modes.some((entry) => entry.id === mode) ? <option value={mode}>{mode || 'Choose a mode'}</option> : null}{program?.modes.map((entry) => <option key={entry.id} value={entry.id}>{entry.meaning}</option>)}</select></label>
    <label className="field">System prompt this agent uses<select name="prompt" value={prompt} onChange={(event) => { const value = event.target.value; if (value === 'keep' || value === 'append' || value === 'replace') { setPrompt(value); setPromptChanged(true); } }}>{supported.map((entry) => <option key={entry} value={entry}>{entry === 'keep' ? "Keep the program’s own prompt" : entry === 'append' ? "Add to the program’s prompt" : "Replace the program’s prompt"}</option>)}</select></label>
    {prompt !== 'keep' ? <label className="field">{prompt === 'replace' ? 'Prompt this agent uses instead' : 'Words added to this agent’s prompt'}<span className="hint">Optional.</span><textarea name="instructions" rows={4} value={instructions} onChange={(event) => setInstructions(event.target.value)} /></label> : null}
    <p>Lys makes this agent’s own folder when it starts.</p>
    <StartAgent agent={id} profile={profile} settings={settings} refusal={refusal} canSave={!readOnly} />
  </section>;
}
