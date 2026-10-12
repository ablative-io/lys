/**
 * What else an agent's settings give it, beside its permissions: the skills
 * Lys keeps that it is handed, the tools its settings name, and what its
 * settings do to the program's own prompt. Skills are a table of the ones
 * Lys keeps with a tick beside each; named tools are a table with Remove.
 *
 * - A skill is named from the ones Lys keeps (GET /skills); a save pins each
 *   to the text Lys keeps then, and refuses a name Lys keeps no text for
 *   (crates/lys-identity-server/src/provisioning_store.rs, `pins`).
 * - A tool named in the settings is added to the rules the agent may use
 *   without asking (crates/lys-identity-server/src/launch_permissions.rs,
 *   `settings`). This screen adds none; it shows the ones there and lets
 *   each be removed.
 */
import { useEffect, useState } from 'react';
import { Refused, request } from '../../api';
import { Act } from '../../shell/Act';
import type { InstalledPlugins } from './choices';
const count = (n: number, one: string, many: string) => n + ' ' + (n === 1 ? one : many);

/** The skills that reach a run beside the ones Lys gives, said only for a program it is known of (CLEAN-START.md rows 10 and 16). */
export function otherSkills(program: string): string {
  if (program === 'Claude Code') return 'Claude Code’s own built-in skills are there as well.';
  if (program === 'Codex') return 'Codex also uses the skills set up for it on the agent’s computer.';
  return '';
}

export function Skills({ program, kept, value, change }: { program: string; kept: string[] | null; value: string[]; change: (next: string[]) => void }) {
  const unknown = kept ? value.filter((name) => !kept.includes(name)) : [];
  const offered = kept ?? [];
  return <section className="given-skills wide" aria-label="Skills">
    <table className="usage-table" role="group" aria-label="Skills Lys keeps">
      <colgroup><col style={{ width: '14%' }} /><col style={{ width: '86%' }} /></colgroup>
      <thead><tr><th>Given</th><th>Skills this agent is given</th></tr></thead>
      <tbody>
        {offered.map((name) => <tr key={name}>
          <td><input type="checkbox" name="skill" value={name} aria-label={'Give ' + name} checked={value.includes(name)}
            onChange={(event) => change(event.target.checked ? [...value, name] : value.filter((one) => one !== name))} /></td>
          <td>{name}</td>
        </tr>)}
        {unknown.map((name) => <tr key={name}><td><Act symbol="remove" name={'Remove ' + name} onClick={() => change(value.filter((one) => one !== name))} /></td>
          <td className="why-not">Lys keeps no skill named {name}, so these settings cannot be saved until it is removed.</td></tr>)}
        {kept === null ? <tr><td colSpan={2} className="why-not">Lys could not list the skills it keeps, so none can be chosen here.</td></tr>
          : offered.length === 0 ? <tr><td colSpan={2} className="dim">Lys keeps no skills yet.</td></tr> : null}
      </tbody>
    </table>
    <p className="dim">{value.length ? count(value.length, 'skill', 'skills') + ': ' + value.join(', ') + '.' : 'None from Lys.'}{otherSkills(program) ? ' ' + otherSkills(program) : ''}</p>
  </section>;
}

/** The tools the settings name. Shown only when there are any; this screen adds none. */
export function ToolsNamed({ value, change }: { value: string[]; change: (next: string[]) => void }) {
  if (value.length === 0) return null;
  return <section className="given-tools wide" aria-label="Tools named in these settings">
    <table className="usage-table">
      <thead><tr><th>Tools named in these settings</th><th>Change</th></tr></thead>
      <tbody>{value.map((name) => <tr key={name} className="rule-row"><td><code>{name}</code></td>
        <td><Act symbol="remove" name={'Remove ' + name} onClick={() => change(value.filter((one) => one !== name))} /></td></tr>)}</tbody>
    </table>
    <p className="dim">Each is added to what this agent may use without asking.</p>
  </section>;
}

/** The Claude Code plugins the settings turn on, each as name@marketplace, and whether each is installed in the
 * config folder a run on the agent's own computer uses, as that computer's runner reads it by the rule its start
 * check uses (plugin_not_installed). Shown only when there are any; this screen adds none. */
export function PluginsNamed({ value, change, machine, machineName }: { value: string[]; change: (next: string[]) => void; machine: string; machineName: string }) {
  const [read, setRead] = useState<{ asked: string; installed?: InstalledPlugins; failed?: string } | null>(null);
  const asked = machine && value.length ? machine : '';
  useEffect(() => {
    if (!asked) return undefined;
    let current = true;
    request<InstalledPlugins>('/harnesses/plugins?machine=' + encodeURIComponent(asked)).then(
      (installed) => { if (current) setRead({ asked, installed }); },
      (error: unknown) => { if (current) setRead({ asked, failed: error instanceof Refused ? error.refusal.refusal + ': ' + error.refusal.reason : String(error) }); },
    );
    return () => { current = false; };
  }, [asked]);
  if (value.length === 0) return null;
  const answer = read && read.asked === asked ? read : null;
  const missing = answer?.installed ? value.filter((name) => !answer.installed?.installed.includes(name)) : [];
  return <section className="given-plugins wide" aria-label="Plugins named in these settings">
    <table className="usage-table">
      <thead><tr><th>Plugins named in these settings</th><th>Change</th></tr></thead>
      <tbody>{value.map((name) => <tr key={name} className="rule-row"><td><code>{name}</code></td>
        <td><Act symbol="remove" name={'Remove ' + name} onClick={() => change(value.filter((one) => one !== name))} /></td></tr>)}</tbody>
    </table>
    <p className="dim">Each is turned on in the settings Lys writes for this agent’s runs. A plugin must already be installed for the program to load it.</p>
    {!machine ? <p className="why-not">No computer is chosen yet, so whether these are installed cannot be read. The start checks on the computer it runs on, and refuses one that is not installed (plugin_not_installed).</p>
      : !answer ? <p className="dim">Reading the plugins installed on {machineName}…</p>
      : answer.failed ? <p className="why-not">The plugins installed on {machineName} could not be read: {answer.failed}. A start there still checks, and refuses one that is not installed (plugin_not_installed).</p>
      : missing.length ? <p className="why-not">Not installed in {answer.installed?.config_dir} on {machineName}: {missing.join(', ')}. A start there is refused (plugin_not_installed) until each is installed in that folder.</p>
      : <p className="dim">Each is installed in {answer.installed?.config_dir} on {machineName}.</p>}
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
