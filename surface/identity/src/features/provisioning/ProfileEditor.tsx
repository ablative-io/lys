/** The agent's settings, as the form it is changed in: each label says what the start does with the value; choices come from the service, never typed as JSON. */
import { useState } from 'react';
import type { FormEvent } from 'react';
import { operationId } from '../../api';
import { useRoleChange } from '../roles/useRoleChange';
import { ChangeStatus } from '../roles/ChangeStatus';
import type { Choices } from './choices';
import type { ProvisioningAnswer, ProvisioningProfile } from './Provisioning';
import { ServiceRows, draftOf, serverOf } from './ServiceRows';
import type { ServiceDraft } from './ServiceRows';

const lines = (text: string) => text.split('\n').map((line) => line.trim()).filter(Boolean);
const unique = (values: string[]) => values.filter((value, index) => values.indexOf(value) === index);

export function ProfileEditor({ id, path, person, profile, choices, changed, readOnly }: { id: string; path: string; person: string; profile: ProvisioningProfile | null; choices: Choices; changed: () => void; readOnly: boolean }) {
  const held = profile?.permissions;
  const declared = profile?.harness ?? null;
  const [programName, setProgramName] = useState(declared?.name ?? '');
  const [build, setBuild] = useState(declared ? declared.program + '\n' + declared.package : '');
  const [models, setModels] = useState<string[]>(profile?.model_access ?? []);
  const [skills, setSkills] = useState<string[]>(profile?.skills ?? []);
  const [services, setServices] = useState<ServiceDraft[]>((profile?.mcp_servers ?? []).map(draftOf));
  const [error, setError] = useState('');
  const program = choices.programs?.find((entry) => entry.name === programName) ?? null;
  const change = useRoleChange<ProvisioningAnswer>('lys.pending.provisioning.' + person + '.' + id, path,
    (answer, body) => answer.agent === id && (answer.recorded ? answer.recorded.operation === body.operation && answer.recorded.version === Number(body.from_version) + 1 : answer.profile !== null && answer.profile.operation === body.operation && answer.profile.version === Number(body.from_version) + 1), changed);
  const harness = () => {
    if (!programName) return null;
    if (!program) {
      if (declared && declared.name === programName) return declared;
      throw new Error('Lys does not list the program ' + programName + ', so it cannot be chosen.');
    }
    const [binary, pkg] = build.split('\n');
    if (!binary) throw new Error('Choose which installed copy of ' + program.name + ' this agent uses.');
    return { name: program.name, description: program.description, program: binary, package: pkg ?? '' };
  };
  const submit = (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault(); if (change.blocked) return;
    try {
      const data = new FormData(event.currentTarget);
      const text = (name: string) => String(data.get(name) ?? '').trim();
      const mcp_servers = services.map(serverOf);
      if (new Set(mcp_servers.map((entry) => entry.name)).size !== mcp_servers.length) throw new Error('Give each connected service a different name.');
      const mode = text('mode');
      const permissions = { allow: unique(lines(text('allow'))), deny: lines(text('deny')), ask: lines(text('ask')), additional_directories: lines(text('folders')), ...(mode ? { default_mode: mode } : {}) };
      const empty = !mode && [permissions.allow, permissions.deny, permissions.ask, permissions.additional_directories].every((list) => !list.length);
      const runsOn = text('runs_on'); const writable = text('writable');
      setError('');
      change.submit({ operation: operationId(), from_version: profile?.version ?? 0, model_access: models.length || !program?.models.length ? models : [program.models[0].id], tools: [], skills, mcp_servers,
        instructions: text('instructions'), note: text('note'), harness: harness(), permissions: empty ? null : permissions,
        ...(runsOn ? { runs_on: runsOn } : {}), ...(writable ? { writable } : {}) });
    } catch (failure) { setError(String(failure)); }
  };
  const toggle = (list: string[], value: string, on: boolean) => on ? unique([...list, value]) : list.filter((entry) => entry !== value);
  const modelChoices = program?.models ?? models.map((model) => ({ id: model, label: model }));
  return <form className="card recorded-form" aria-label="This agent's settings" onSubmit={submit}>
    <fieldset disabled={readOnly || change.blocked} style={{ border: 0, padding: 0 }}>
      <label className="field">Computer<span className="hint">Which computer this agent works on. It starts there unless you pick another when you start it.</span>
        <select name="runs_on" defaultValue={profile?.runs_on ?? ''}><option value="">Chosen at each start</option>{choices.machines.map((machine) => <option key={machine.id} value={machine.id}>{machine.name}</option>)}</select></label>
      <label className="field">Folder it may change<span className="hint">The one folder this agent can change. Leave empty and it can change anything your account can. Not enforced yet.</span>
        <input name="writable" defaultValue={profile?.writable ?? ''} placeholder="/Users/you/agents/this-agent" /></label>
      <label className="field">Program<span className="hint">The program that does this agent's work. A start is refused until one is chosen.</span>
        <select value={programName} onChange={(event) => { setProgramName(event.target.value); setBuild(''); setModels([]); }}><option value="">Not chosen</option>
          {(choices.programs ?? (declared ? [{ name: declared.name, line: '' }] : [])).map((entry) => <option key={entry.name} value={entry.name}>{entry.name}{entry.line ? ' — ' + entry.line : ''}</option>)}</select>
        {choices.programs === null ? <span className="hint">Lys cannot list its programs yet ({choices.programsMissing}); the program already saved is kept.</span> : null}</label>
      {program ? <label className="field">Installed copy<span className="hint">Where {program.name} is on the computer, as Lys found it. The start runs exactly this file.</span>
        <select value={build} onChange={(event) => setBuild(event.target.value)}><option value="">Choose</option>{program.builds.map((entry) => <option key={entry.program + entry.package} value={entry.program + '\n' + entry.package}>{entry.program} · {entry.package}</option>)}</select>
        {!program.builds.length ? <span className="hint">No computer has reported where {program.name} is installed yet.</span> : null}</label> : null}
      <label className="field">Model<span className="hint">Which AI model it uses, passed to the program as --model.</span>
        <select value={models[0] ?? (program?.models[0]?.id ?? '')} onChange={(event) => setModels(unique([event.target.value, ...models.slice(1)].filter(Boolean)))}>{program ? null : <option value="">The program's default</option>}{modelChoices.map((model) => <option key={model.id} value={model.id}>{model.label}</option>)}</select></label>
      {models[0] ? <div className="field">Backup models<span className="hint">Used in order if the first is unavailable, passed as --fallback-model.</span>
        {modelChoices.filter((model) => model.id !== models[0]).map((model) => <label key={model.id}><input type="checkbox" checked={models.slice(1).includes(model.id)} onChange={(event) => setModels([models[0], ...toggle(models.slice(1), model.id, event.target.checked)])} /> {model.label}</label>)}</div> : null}
      <label className="field">Standing instructions<span className="hint">Written to instructions.md and added to the end of the program's own system prompt each time the agent starts. They do not replace it.</span>
        <textarea name="instructions" rows={6} defaultValue={profile?.instructions ?? ''} /></label>
      <div className="field">Skills<span className="hint">At each start Lys writes each chosen skill into the agent's own settings folder as skills/name/SKILL.md, as the text saved with these settings. The program sees each skill's name and summary when it starts and reads the whole text when a job matches it.</span>
        {choices.skills === null ? skills.map((skill) => <span key={skill}>{skill}</span>) : choices.skills.map((skill) => <label key={skill}><input type="checkbox" checked={skills.includes(skill)} onChange={(event) => setSkills(toggle(skills, skill, event.target.checked))} /> {skill}</label>)}</div>
      <div className="field">Connected services<span className="hint">Other programs this agent can use, written into the start's MCP settings file.</span>
        <ServiceRows drafts={services} setDrafts={setServices} secrets={choices.secrets} /></div>
      <label className="field">Can do without asking<span className="hint">Written into the agent's settings file as its allow rules: one per line, a tool's name, or a tool's name with one detail in brackets.</span>
        <textarea name="allow" rows={3} defaultValue={unique([...(held?.allow ?? []), ...(profile?.tools ?? [])]).join('\n')} /></label>
      <label className="field">Must ask first<span className="hint">Written as its ask rules: it stops and asks before doing these.</span>
        <textarea name="ask" rows={2} defaultValue={held?.ask?.join('\n') ?? ''} /></label>
      <label className="field">Never allowed<span className="hint">Written as its deny rules: refused even when asked.</span>
        <textarea name="deny" rows={2} defaultValue={held?.deny?.join('\n') ?? ''} /></label>
      <label className="field">How much it decides alone<span className="hint">The program's permission mode.</span>
        <select name="mode" key={program?.name ?? ''} defaultValue={held?.default_mode ?? program?.modes[0]?.id ?? ''}>{program ? null : <option value="">The program's default</option>}
          {(program?.modes ?? (held?.default_mode ? [{ id: held.default_mode, meaning: '' }] : [])).map((mode) => <option key={mode.id} value={mode.id}>{mode.meaning ? mode.id + ' — ' + mode.meaning : mode.id}</option>)}</select></label>
      <label className="field">Other folders it may work in<span className="hint">Folders outside its own it can open and use, one per line. Written into its settings file as additional directories.</span>
        <textarea name="folders" rows={2} defaultValue={held?.additional_directories?.join('\n') ?? ''} /></label>
      {readOnly ? <p className="note">Only a directory administrator can change these settings.</p> : <><label className="field">What you changed and why<span className="hint">Kept with this version of the settings.</span><input name="note" required /></label>
      <button type="submit" className="btn primary">Save these settings</button></>}
    </fieldset>{error ? <p role="alert">{error}</p> : null}<ChangeStatus change={change} />
  </form>;
}
