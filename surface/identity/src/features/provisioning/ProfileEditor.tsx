import { useEffect, useState } from 'react';
import type { ReactNode } from 'react';
import type { Choices } from './choices';
import type { ProvisioningProfile } from './Provisioning';
import { FolderChooser } from './FolderChooser';
import { ModeWords } from './ModeWords';
import { Permissions } from './Permissions';
import type { Permissions as PermissionsValue } from './Provisioning';
import { usable } from '../network/allowed';
import { SaveSettings } from './SaveSettings';
import { McpServers } from './McpServers';
import { Skills, ToolsNamed, PluginsNamed, promptWords } from './Given';
import type { PeopleView } from '../../generated';
import { ErrorWords, problemWords } from '../people/Words';
import { copyProgram, copyValue, distinctCopies, ProfileCopiesProblems, useProfileCopies } from './ProfileCopies';

import { commandOrAbsolutePath, launchCommand, PROGRAM_RULE } from './harness-program';
export { commandOrAbsolutePath as isCommandOrPath } from './harness-program';

export function ProfileEditor({ id, profile, choices, people, readOnly = false, saved }: {
  id: string; profile: ProvisioningProfile | null; choices: Choices; people: PeopleView; readOnly?: boolean;
  /** Called once the settings are saved, when the form is the set-up before a start. */
  saved?: () => void;
}) {
  return <ProfileFields profile={profile} choices={choices} canChoose={!readOnly} agent={id} firstRun={Boolean(saved) && !profile?.harness} folderFirst={Boolean(saved)} render={(fields, settings, refusal) => <section className="card" aria-label="Settings of this agent">
    {fields}<SaveSettings agent={id} profile={profile} settings={settings}
      refusal={refusal || (typeof settings.working_folder === 'string' && settings.working_folder.trim() ? ''
        : "WorkingFolderUnnamed: the launch names no working folder and the agent's profile has no working_folder; name one on the launch or set the agent's default")}
      canSave={!readOnly} people={people} machines={choices.machines} saved={saved} />
  </section>} />;
}

export function ProfileFields({ profile, choices, strict = false, firstRun = false, folderFirst = false, computer, canChoose = true, agent, brief = false, newComputer = false, pin, more, render }: {
  /** Only the model and the computer are shown; everything else is kept as saved. */
  brief?: boolean;
  profile: ProvisioningProfile | null; choices: Choices; strict?: boolean; firstRun?: boolean;
  /** The folder is asked first and must be chosen: the agent is started straight after the save. */
  folderFirst?: boolean;
  /** The computer the agent is to run on, when the screen already knows it; the folder is chosen there. */
  computer?: string; canChoose?: boolean;
  /** The agent these settings belong to, when it exists; the rules its policy forces are shown for it. */
  agent?: string;
  /** The agent runs on a computer not added yet: no runner can report its copies, so the copies Lys lists are offered. */
  newComputer?: boolean;
  /** Told the computer of a copy picked from another computer's group, so the screen around it runs the agent there. */
  pin?: (machine: string) => void;
  /** Further choices of the screen around, shown with this form's own under More settings on a first run. */
  more?: ReactNode;
  render: (fields: ReactNode, settings: Record<string, unknown>, refusal: string) => ReactNode;
}) {
  const [runsOn, setRunsOn] = useState(profile?.runs_on ?? '');
  // The screen's computer, when it names one, wins over a copy picked earlier from another computer's group.
  const preferred = computer || runsOn;
  const machines = usable(choices.machines);
  const { catalogues, pending: readingCopies } = useProfileCopies(machines, preferred);
  const catalogue = preferred ? catalogues.find((entry) => entry.machine.id === preferred) : undefined;
  const available = preferred && !newComputer ? catalogue?.programs ?? [] : machines.length === 1 && catalogues.length === 1 ? catalogues[0].programs : choices.programs ?? [];
  const workspaceMode = (entry: typeof available[number] | undefined) => entry?.modes.find((mode) => mode.id === 'workspace-only' || mode.id === 'workspace-write')?.id ?? '';
  // A first run takes a program the computer already found (by the login's PATH), never asking for a path.
  const initialProgram = firstRun ? available.find((entry) => entry.builds.length > 0 && workspaceMode(entry)) ?? available.find((entry) => entry.builds.length > 0) ?? available[0] : available.length === 1 ? available[0] : undefined;
  const [programName, setProgram] = useState(profile?.harness?.name ?? initialProgram?.name ?? '');
  const program = available.find((entry) => entry.name === programName);
  const [model, setModel] = useState(profile?.model_access[0] ?? (strict && program?.models.length !== 1 ? '' : program?.models[0]?.id ?? ''));
  // A first run keeps the program to its folder where it can; where it cannot, it starts in the program's own default mode.
  const firstMode = (entry: typeof program) => firstRun ? workspaceMode(entry) || (entry?.modes[0]?.id ?? '') : strict && entry?.modes.length !== 1 ? '' : entry?.modes[0]?.id ?? '';
  const firstPrompt = (entry: typeof program) => entry?.instructions_modes?.includes('keep') ? 'keep' : entry?.instructions_modes?.includes('append') ? 'append' : '';
  // The whole of what the agent may do: the mode, the three rule lists and the extra folders. The mode is one member of it.
  const [permissions, setPermissions] = useState<PermissionsValue>(() => ({ ...profile?.permissions, default_mode: profile?.permissions?.default_mode ?? firstMode(program) }));
  const mode = permissions.default_mode ?? '';
  const [prompt, setPrompt] = useState(firstRun ? profile?.instructions_mode ?? firstPrompt(program) : strict ? profile?.instructions_mode ?? (program?.instructions_modes?.length === 1 ? program.instructions_modes[0] : '') : profile?.instructions ? profile.instructions_mode ?? 'append' : 'keep');
  const [promptChanged, setPromptChanged] = useState(false);
  const [folder, setFolder] = useState(profile?.working_folder ?? '');
  // The computer it runs on is a saved setting like the rest; empty leaves it to be settled when the agent starts.
  // Where folders and plugins are read: the computer the profile is saved to run on, else the one the screen already knows.
  const [instructions, setInstructions] = useState(profile?.instructions ?? '');
  // What else the agent is given: the tools its settings name, the skills Lys keeps for it, and its connected tools.
  const [tools, setTools] = useState(profile?.tools ?? []);
  const [skills, setSkills] = useState(profile?.skills ?? []);
  const [plugins, setPlugins] = useState(profile?.plugins ?? []);
  const [servers, setServers] = useState(profile?.mcp_servers ?? []);
  const [requiresControls, setRequiresControls] = useState(profile?.session?.requires_controls ?? false);
  const aggregate = newComputer;
  const groups = (aggregate ? [] : catalogues).map((entry) => ({ machine: entry.machine, builds: entry.programs.find((entry) => entry.name === programName)?.builds ?? [] }));
  const copies = aggregate ? distinctCopies(program?.builds ?? []).map((copy) => ({ ...copy, program: copyProgram(copy), machine: '', value: copyValue(copy) }))
    : groups.flatMap((group) => group.builds.map((copy) => ({ ...copy, program: copyProgram(copy), machine: group.machine.id,
      value: preferred || machines.length === 1 ? copyValue(copy) : group.machine.id + '\n' + copyValue(copy) })));
  const builds = copies;
  const heldBuild = profile?.harness?.name === programName ? profile.harness : null;
  const heldCopy = (copies: typeof builds) => heldBuild ? copies.find((entry) => entry.program === heldBuild.program && entry.package === heldBuild.package)
    ?? copies.find((entry) => launchCommand(entry.program) === launchCommand(heldBuild.program)) : undefined;
  const [build, setBuild] = useState(heldBuild ? 'another' : (aggregate || preferred || machines.length === 1) && (firstRun ? builds.length > 0 : builds.length === 1) ? builds[0].value : '');
  const [programPath, setProgramPath] = useState(heldBuild?.program ?? '');
  useEffect(() => {
    if (readingCopies || !catalogues.length && !aggregate) return;
    const scoped = aggregate || preferred || machines.length === 1;
    const next = available.find((entry) => entry.name === programName) ?? available.find((entry) =>
      catalogues.some((catalogue) => catalogue.programs.some((reported) => reported.name === entry.name && reported.builds.length > 0))) ?? available[0];
    if (!programName && next) {
      setProgram(next.name); setModel(next.models[0]?.id ?? ''); setPermissions({ default_mode: firstMode(next) }); setPrompt(firstPrompt(next));
    }
    const reported = heldCopy(builds);
    if (scoped && reported && build === 'another' && programPath === heldBuild?.program) setBuild(reported.value);
    else if (scoped && !build && (firstRun ? builds.length > 0 : builds.length === 1)) setBuild(builds[0].value);
  }, [catalogues, readingCopies, programName, preferred]);
  const supported = program?.instructions_modes ?? ['keep', 'append'];
  const selected = build === 'another' && commandOrAbsolutePath(programPath)
    ? { program: programPath, package: heldBuild && heldBuild.program === programPath ? heldBuild.package : programPath }
    : builds.find((entry) => entry.value === build);
  const selectedProgram = selected && launchCommand(selected.program);
  let refusal = '';
  if (!preferred && choices.programs === null) refusal = 'This Lys is too old to list programs. It gets the list when Lys is updated.';
  else if (!aggregate && preferred && !machines.some((entry) => entry.id === preferred)) refusal = 'Choose a computer that is still in use and has a connected runner.';
  else if (readingCopies) refusal = preferred ? 'Reading this computer’s programs.' : 'Reading the computers’ installed programs.';
  else if (catalogue?.failed) refusal = 'Lys could not read this computer’s installed programs. Check its runner before choosing a copy.';
  else if (!aggregate && !preferred && catalogues.some((entry) => entry.failed)) refusal = 'Choose a computer whose installed programs Lys could read.';
  else if (catalogue?.problem && !selectedProgram?.startsWith('/')) refusal = problemWords(catalogue.problem);
  else if (!program) refusal = available.length ? 'Choose the program this agent will use.' : 'Lys lists no programs it can start.';
  else if (!selected) refusal = build === 'another' ? PROGRAM_RULE : builds.length === 0
    ? (aggregate ? 'Lys did not find ' + program.name + ' on this computer. Install it and open this page again, or choose “Another program” and type the command you use to start it.'
      : preferred || machines.length === 1 ? program.name + ' is not installed on this computer.'
      : machines.length ? 'No computer reported an installed copy of ' + program.name + '.'
        : 'No computer with a connected runner is available to report installed copies of ' + program.name + '.')
      + (program.not_found ? ' ' + problemWords(program.not_found) : '')
    : 'Choose an installed copy of ' + program.name + '.';
  else if (preferred && !selectedProgram?.startsWith('/') && !builds.some((entry) => launchCommand(entry.program) === selectedProgram)) refusal = 'This computer did not report ' + launchCommand(selected.program) + '. ' + PROGRAM_RULE;
  else if (!program.models.some((entry) => entry.id === model)) refusal = 'Choose a model this program lists.';
  else if (!program.modes.some((entry) => entry.id === mode)) refusal = firstRun ? program.name + ' lists no mode Lys can start it in.' : 'Choose a mode this program lists.';
  else if (!supported.some((entry) => entry === prompt)) refusal = 'This program does not offer that prompt choice.';
  else if (prompt === 'replace' && !instructions.trim()) refusal = 'Type the prompt this agent uses instead, or choose another prompt setting.';
  else if (choices.skills && skills.some((name) => !choices.skills?.includes(name))) refusal = 'Lys keeps no skill named ' + skills.find((name) => !choices.skills?.includes(name)) + '. Remove it from this agent’s skills.';
  else if (folderFirst && !folder) refusal = 'Choose the folder this agent works in.';
  const settings: Record<string, unknown> = {
    model_access: profile?.model_access[0] === model ? profile.model_access : [model],
    tools, skills, ...(profile && profile.plugins === undefined && plugins.length === 0 ? {} : { plugins }), mcp_servers: servers,
    instructions: prompt === 'keep' ? !promptChanged && profile ? profile.instructions : '' : instructions.trim(), note: 'Settings saved from the settings form',
    harness: program && selected ? { name: program.name, description: program.description, program: selected.program, package: selected.package } : null,
    permissions, ...(program?.instructions_modes ? { instructions_mode: !promptChanged && profile ? profile.instructions_mode ?? 'append' : prompt } : {}),
    ...(preferred ? { runs_on: preferred } : {}), ...(profile?.writable ? { writable: profile.writable } : {}),
    ...(folder ? { working_folder: folder } : {}),
    session: requiresControls
      ? { sensitive: false, ...profile?.session, requires_controls: true }
      : profile?.session ? Object.fromEntries(Object.entries(profile.session).filter(([name]) => name !== 'requires_controls')) : profile?.session ?? null,
  };
  const where = <div className="wide">
    {folder ? <p className="works-in">Works in <code>{folder}</code></p> : <p className="works-in">No folder chosen yet.{firstRun && !folderFirst ? ' Choose one now, or when you start this agent.' : ''}</p>}
    {canChoose ? <FolderChooser computers={machines} preferred={preferred} chosen={folder} choose={setFolder} /> : null}
  </div>;
  // The two choices the Overview also offers: the same fields, the same state, the same saver.
  const modelField = <label className="field">Model this agent uses<select name="model" value={model} onChange={(event) => setModel(event.target.value)}>{!program?.models.some((entry) => entry.id === model) ? <option value={model}>{model || 'Choose a model'}</option> : null}{program?.models.map((entry) => <option key={entry.id} value={entry.id}>{entry.label}</option>)}</select></label>;
  const computerField = computer !== undefined ? null : <label className="field">Computer this agent runs on<select name="runs_on" value={runsOn} disabled={!canChoose} onChange={(event) => { setRunsOn(event.target.value); if (build !== 'another') setBuild(''); }}>
      <option value="">Settled when it starts</option>
      {runsOn && !machines.some((entry) => entry.id === runsOn) ? <option value={runsOn}>A computer Lys no longer lists</option> : null}
      {machines.map((entry) => <option key={entry.id} value={entry.id}>{entry.name}</option>)}
    </select></label>;
  const programField = <>
    <label className="field">Program this agent uses<select name="program" value={programName} onChange={(event) => {
      const next = available.find((entry) => entry.name === event.target.value);
      setProgram(event.target.value); setModel(strict && next?.models.length !== 1 ? '' : next?.models[0]?.id ?? ''); setPermissions({ default_mode: firstMode(next), ...(permissions.additional_directories ? { additional_directories: permissions.additional_directories } : {}) }); setPrompt(firstRun ? firstPrompt(next) : strict ? next?.instructions_modes?.length === 1 ? next.instructions_modes[0] : '' : 'keep'); setPromptChanged(true);
      const nextCopies = aggregate ? distinctCopies(next?.builds ?? []) : catalogues.flatMap((catalogue) => catalogue.programs.find((entry) => entry.name === next?.name)?.builds ?? []);
      setBuild((aggregate || preferred || machines.length === 1) && (firstRun ? nextCopies.length > 0 : nextCopies.length === 1) ? copyValue(nextCopies[0]) : '');
      setProgramPath('');
    }}>
      {!available.some((entry) => entry.name === programName) ? <option value={programName}>{programName || 'Choose a program'}</option> : null}
      {available.map((entry) => <option key={entry.name} value={entry.name}>{entry.name}</option>)}
    </select></label>
  </>;
  const copyField = <>
    {program ? <label className="field">Installed copy this agent uses<select name="build" value={build} onChange={(event) => {
      const chosen = copies.find((entry) => entry.value === event.target.value);
      if (chosen) { if (chosen.machine) { setRunsOn(chosen.machine); pin?.(chosen.machine); } setBuild(copyValue(chosen)); setProgramPath(''); }
      else setBuild(event.target.value);
    }}><option value="">Choose an installed copy</option>{aggregate ? copies.map((entry) => <option key={entry.value} value={entry.value}>{entry.name} — {launchCommand(entry.program)}</option>) : null}{groups.filter((group) => group.builds.length > 0).map((group) => <optgroup key={group.machine.id} label={group.machine.name}>{copies.filter((copy) => copy.machine === group.machine.id).map((entry) => <option key={entry.value} value={entry.value}>{entry.name} — {launchCommand(entry.program)}</option>)}</optgroup>)}<option value="another">Another program (type its command)…</option></select></label> : null}
    <ProfileCopiesProblems catalogues={catalogues} />
    {program?.not_found && !builds.length ? <div className="wide"><ErrorWords problem={program.not_found} /></div> : null}
    {build === 'another' ? <label className="field">Command or program path<input name="program-path" value={programPath} onChange={(event) => setProgramPath(event.target.value)} /><span className="hint">Only needed when Lys did not find the program itself. Type the command you use to start it in Terminal, such as claude, or its full path, such as /opt/homebrew/bin/claude.</span></label> : null}
  </>;
  const restFields = <>
    {modelField}
    {computerField}
    {firstRun && profile && !program?.modes.some((entry) => entry.id === mode) ? <p>The saved program settings are kept for this request.</p> : null}
    <Permissions key={programName} agent={agent} program={program} value={permissions} change={setPermissions} tools={tools} computers={machines} computer={preferred} />
    <div className="wide"><ModeWords program={program} mode={mode} sentencesOnly /></div>
    {/* No program's managed adapter is reported qualified here, so the requirement is offered only where it is already set, to be turned off. */}
    {profile?.session?.requires_controls ? <label className="field"><span>Require automatic context and reminder controls</span><input type="checkbox" name="requires_controls" checked={requiresControls} disabled={!canChoose} onChange={(event) => setRequiresControls(event.target.checked)} /><span className="hint">Starts are refused until this program’s managed adapter is qualified.</span></label> : null}
    <ToolsNamed value={tools} change={setTools} />
    <PluginsNamed value={plugins} change={setPlugins} machine={preferred ?? ''} machineName={preferred ? machines.find((entry) => entry.id === preferred)?.name ?? preferred : ''} />
    <McpServers key={'servers:' + programName} program={program?.name ?? ''} transports={program?.description?.mcp?.transports} value={servers} change={setServers} />
    <Skills program={program?.name ?? ''} kept={choices.skills} value={skills} change={setSkills} />
    <label className="field">System prompt this agent uses<select name="prompt" value={prompt} onChange={(event) => { const value = event.target.value; if (value === 'keep' || value === 'append' || value === 'replace') { setPrompt(value); setPromptChanged(true); } }}>{!supported.some((entry) => entry === prompt) ? <option value="">Choose a prompt</option> : null}{supported.map((entry) => <option key={entry} value={entry}>{entry === 'keep' ? "Keep the program’s own prompt" : entry === 'append' ? "Add to the program’s prompt" : "Replace the program’s prompt"}</option>)}</select></label>
    {prompt ? <p className="dim prompt-words">{promptWords(prompt, prompt === 'keep' ? '' : instructions)}</p> : null}
    {prompt && prompt !== 'keep' ? <label className="field wide">{prompt === 'replace' ? 'Prompt this agent uses instead' : 'Words added to this agent’s prompt'}<span className="hint">Optional.</span><textarea name="instructions" rows={4} value={instructions} onChange={(event) => setInstructions(event.target.value)} /></label> : null}
  </>;
  // A first run asks only what it must: the program (its found copy chosen for the person) and the folder; the rest waits under More settings.
  const found = firstRun && build !== 'another' && builds.some((entry) => entry.value === build) ? selected : undefined;
  const fallbackMode = firstRun && program && !workspaceMode(program) ? program.modes.find((entry) => entry.id === mode) : undefined;
  const fields = firstRun ? <div className="form-grid">
    {folderFirst ? where : null}
    {programField}
    {found ? <p className="wide first-run-copy">Lys found {program?.name} on this computer and uses <code>{launchCommand(found.program)}</code>.</p> : copyField}
    {fallbackMode ? <p className="wide hint first-run-mode">{program?.name} has no mode that keeps it to its own folder with the internet off, so it starts in its own default mode ({fallbackMode.meaning}). You can change this under More settings.</p> : null}
    {folderFirst ? null : where}
    <details className="wide more-settings"><summary>More settings</summary><div className="form-grid">
      {found ? copyField : null}
      {restFields}
      {more}
    </div></details>
  </div> : <div className="form-grid">
    {folderFirst ? where : null}
    {programField}
    {copyField}
    {restFields}
    {folderFirst ? null : where}
  </div>;
  return render(brief ? <div className="form-grid">{modelField}{computerField}</div> : fields, settings, refusal);
}
