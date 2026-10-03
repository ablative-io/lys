/**
 * What else an agent's settings give it, beside its permissions: the skills
 * Lys keeps that it is handed, the tools its settings name, and what its
 * settings do to the program's own prompt. Each is one short section that
 * says what is there now in a sentence, and opens only when asked.
 *
 * - A skill is named from the ones Lys keeps (GET /skills); a save pins each
 *   to the text Lys keeps then, and refuses a name Lys keeps no text for
 *   (crates/lys-identity-server/src/provisioning_store.rs, `pins`).
 * - A tool named in the settings is added to the rules the agent may use
 *   without asking (crates/lys-identity-server/src/launch_permissions.rs,
 *   `settings`). This screen adds none; it shows the ones there and lets
 *   each be removed.
 */
import { useState } from 'react';

const count = (n: number, one: string, many: string) => n + ' ' + (n === 1 ? one : many);

export function Skills({ kept, value, change }: { kept: string[] | null; value: string[]; change: (next: string[]) => void }) {
  const [open, setOpen] = useState(false);
  const unknown = kept ? value.filter((name) => !kept.includes(name)) : [];
  const offered = kept ?? [];
  return <section className="rule-list given-skills" aria-label="Skills">
    <p><b>Skills this agent is given</b></p>
    <p className="dim">{value.length ? count(value.length, 'skill', 'skills') + ': ' + value.join(', ') + '.' : 'None.'}</p>
    {unknown.map((name) => <p key={name} className="why-not">Lys keeps no skill named {name}, so these settings cannot be saved until it is removed.{' '}
      <button type="button" className="btn" onClick={() => change(value.filter((one) => one !== name))}>Remove {name}</button></p>)}
    {kept === null ? <p className="why-not">Lys could not list the skills it keeps, so none can be chosen here.</p>
      : offered.length === 0 ? <p className="dim">Lys keeps no skills yet.</p>
      : open ? <div role="group" aria-label="Skills Lys keeps">
        {offered.map((name) => <label key={name} className="choice-row"><input type="checkbox" name="skill" value={name} checked={value.includes(name)}
          onChange={(event) => change(event.target.checked ? [...value, name] : value.filter((one) => one !== name))} /> {name}</label>)}
        <p><button type="button" className="btn" onClick={() => setOpen(false)}>Done</button></p>
      </div>
      : <p><button type="button" className="btn" onClick={() => setOpen(true)}>Choose skills</button></p>}
  </section>;
}

/** The tools the settings name. Shown only when there are any; this screen adds none. */
export function ToolsNamed({ value, change }: { value: string[]; change: (next: string[]) => void }) {
  if (value.length === 0) return null;
  return <section className="rule-list given-tools" aria-label="Tools named in these settings">
    <p><b>Tools named in these settings</b></p>
    <p className="dim">Each is added to what this agent may use without asking.</p>
    <ul>{value.map((name) => <li key={name} className="rule-row"><code>{name}</code>{' '}
      <button type="button" className="btn" onClick={() => change(value.filter((one) => one !== name))}>Remove {name}</button></li>)}</ul>
  </section>;
}

/** What the settings do to the program's own prompt, in one sentence. */
export function promptWords(mode: string, words: string): string {
  const typed = words.trim().length > 0;
  if (mode === 'keep') return 'This agent uses the program’s own prompt. These settings add nothing to it.';
  if (mode === 'append') return typed ? 'The words below are added to the program’s own prompt. Nothing of the program’s prompt is taken away.' : 'Nothing is added yet: the box below is empty, so this agent uses the program’s own prompt.';
  if (mode === 'replace') return typed ? 'The words below are used instead of the program’s own prompt. The program’s own prompt is not used.' : 'The program’s own prompt is to be replaced, and the box below is empty. Type the prompt this agent uses instead.';
  return '';
}
