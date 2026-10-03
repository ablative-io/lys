/**
 * What an agent may do, on the settings form and the add form. It edits what
 * Lys already stores and writes into the start: the mode, the three rule
 * lists and the extra folders of the agent's settings.
 *
 * The front is a short column of choices, one per mode the chosen program
 * lists, each with a plain name and one sentence; a person gets a sensible
 * agent without opening anything else. Under it one line sums the rules, and
 * "Show every rule" opens them in place. A rule is added by choosing its
 * kind and giving the one thing that kind needs; the program's own grammar
 * is never typed, and each rule is shown in plain words with the exact rule
 * small beside it. Nothing here says a rule is enforced.
 */
import { useState } from 'react';
import { request, useLoad } from '../../api';
import type { PolicyView } from '../file/policyContract';
import type { Program } from './choices';
import { FolderChooser } from './FolderChooser';
import { FIRST, LISTS, MODES, firstSentence, forcedBy } from './permission-modes';
import type { RuleList } from './permission-modes';
import { RULE_KINDS, ruleFor, summary, wordsFor } from './permission-rules';
import type { RuleKind } from './permission-rules';
import type { Permissions as Value } from './Provisioning';

type Computers = { id: string; name: string }[];

/** The mode Lys makes itself, which cannot carry rules under Without asking or extra folders (crates/lys-home/src/harness/claude_code/launch_env.rs, lines 75 to 86). */
const KEPT = 'workspace-only';

function Mode({ entry, chosen, pick }: { entry: Program['modes'][number]; chosen: boolean; pick: () => void }) {
  const [more, setMore] = useState(false);
  const known = MODES[entry.id];
  const { first, rest } = firstSentence(entry.meaning);
  return <div className="mode-row">
    <label><input type="radio" name="permission-mode" value={entry.id} checked={chosen} onChange={pick} /> <b>{known?.name ?? entry.id}</b>{known ? <small className="mode-id">{' ' + entry.id}</small> : null}{known?.warn ? <span className="mode-warn" role="img" aria-label="Warning"> ⚠</span> : null}</label>
    <p className="mode-meaning">{first}{rest && more ? ' ' + rest : ''}{rest && !more ? <> <button type="button" className="link" onClick={() => setMore(true)}>more</button></> : null}</p>
  </div>;
}

/** The rules the agent's policy forces, read only when the lists are opened; locked, and said to be the policy's. */
function Forced({ agent }: { agent: string }) {
  const load = useLoad(() => request<PolicyView>('/agents/' + encodeURIComponent(agent) + '/policy'), 'permissions-policy:' + agent);
  if (load.status === 'loading') return null;
  if (load.status === 'refused') return <li className="rule-row">Lys could not read this agent’s policy, so the rules it forces are not shown. {load.refused.message} <small className="refusal-name">{load.refused.refusal.refusal}</small></li>;
  return <>{forcedBy(load.data.policy?.rules ?? []).map((rule) => <li key={'forced ' + rule} className="rule-row locked">{wordsFor(rule)} <small><code>{rule}</code></small> <span className="dim">from this agent’s policy</span></li>)}</>;
}

/** One rule, step by step in place: the kind, then the one thing that kind needs, then the list it goes on. */
function AddRule({ lists, computers, computer, add, done }: {
  lists: RuleList[]; computers: Computers; computer: string; add: (list: RuleList, rule: string) => void; done: () => void;
}) {
  const [kind, setKind] = useState<RuleKind | null>(null);
  const [given, setGiven] = useState('');
  const [list, setList] = useState<RuleList | null>(null);
  const built = kind ? ruleFor(kind, given) : null;
  const rule = built && 'rule' in built ? built.rule : null;
  const problem = built && 'problem' in built ? built.problem : '';
  const folder = kind === 'read-folder' || kind === 'change-folder';
  return <div className="add-rule" role="group" aria-label="Add a rule">
    <p><b>What is the rule about?</b></p>
    {RULE_KINDS.map(([id, words]) => <label key={id} className="choice-row"><input type="radio" name="rule-kind" value={id} checked={kind === id} onChange={() => { setKind(id); setGiven(''); }} /> {words}</label>)}
    {folder ? <>
      {given ? <p className="works-in">The folder <code>{given}</code></p> : <p>Choose the folder.</p>}
      <FolderChooser computers={computers} preferred={computer} chosen={given} choose={setGiven} label={given ? 'Choose a different folder for this rule' : 'Choose the folder for this rule'} confirm="Use" />
    </> : null}
    {kind === 'command' ? <label className="field">The words the command starts with<span className="hint">For example: git status</span><input name="rule-command" value={given} onChange={(event) => setGiven(event.target.value)} onKeyDown={(event) => { if (event.key === 'Enter') event.preventDefault(); }} /></label> : null}
    {kind === 'website' ? <label className="field">The website’s name<span className="hint">For example: example.org</span><input name="rule-host" value={given} onChange={(event) => setGiven(event.target.value)} onKeyDown={(event) => { if (event.key === 'Enter') event.preventDefault(); }} /></label> : null}
    {kind === 'tool' ? <label className="field">The tool’s name, exactly as the program names it<span className="hint">For example: WebSearch</span><input name="rule-tool" value={given} onChange={(event) => setGiven(event.target.value)} onKeyDown={(event) => { if (event.key === 'Enter') event.preventDefault(); }} /></label> : null}
    {kind && given.trim() && problem ? <p className="why-not">{problem}</p> : null}
    {rule ? <>
      <p><b>What happens when it tries this?</b></p>
      {LISTS.filter((entry) => lists.includes(entry.id)).map((entry) => <label key={entry.id} className="choice-row"><input type="radio" name="rule-list" value={entry.id} checked={list === entry.id} onChange={() => setList(entry.id)} /> {entry.name}</label>)}
      <p className="rule-row">{wordsFor(rule)} <small><code>{rule}</code></small></p>
    </> : null}
    <p>
      <button type="button" className="btn primary" disabled={!rule || !list} onClick={() => { if (rule && list) { add(list, rule); done(); } }}>Add this rule</button>{' '}
      <button type="button" className="btn" onClick={done}>Cancel</button>
    </p>
    {!kind ? <p className="why-not">Choose what the rule is about.</p> : !rule ? given.trim() ? null : <p className="why-not">{problem}</p> : !list ? <p className="why-not">Choose what happens.</p> : null}
  </div>;
}

export function Permissions({ agent, program, value, change, computers, computer }: {
  /** The agent whose policy forces rules; none while an agent is being added. */
  agent?: string; program: Program | undefined; value: Value; change: (next: Value) => void; computers: Computers; computer: string;
}) {
  const mode = value.default_mode ?? '';
  const modes = program?.modes ?? [];
  const front = FIRST.flatMap((id) => modes.filter((entry) => entry.id === id));
  const behind = modes.filter((entry) => !FIRST.includes(entry.id));
  const [moreModes, setMoreModes] = useState(() => behind.some((entry) => entry.id === mode));
  const [open, setOpen] = useState(false);
  const [adding, setAdding] = useState(false);
  if (!program) return null;
  const takesRules = program.description.permissions.rule_forms.length > 0;
  const kept = mode === KEPT;
  const remove = (list: RuleList | 'additional_directories', entry: string) => change({ ...value, [list]: (value[list] ?? []).filter((one) => one !== entry) });
  const add = (list: RuleList | 'additional_directories', entry: string) => { if (!(value[list] ?? []).includes(entry)) change({ ...value, [list]: [...(value[list] ?? []), entry] }); };
  const folders = value.additional_directories ?? [];
  const extraFolders = <section className="rule-list" aria-label="Extra folders">
    <h4>Extra folders</h4>
    {kept ? <p className="dim">Not available with Kept to its folder: that setting keeps the agent to its one working folder.</p> : null}
    {folders.length ? <ul>{folders.map((entry) => <li key={entry} className="rule-row"><code>{entry}</code> <button type="button" className="btn" onClick={() => remove('additional_directories', entry)}>Remove</button></li>)}</ul> : kept ? null : <p className="dim">None. It works in its working folder only.</p>}
    {kept ? null : <FolderChooser computers={computers} preferred={computer} chosen="" choose={(folder) => add('additional_directories', folder)} label="Add an extra folder" confirm="Add" />}
  </section>;
  return <div className="permissions" role="group" aria-label="What this agent may do">
    <p><b>What {program.name} may do</b></p>
    {(moreModes ? [...front, ...behind] : front.length ? front : behind).map((entry) => <Mode key={entry.id} entry={entry} chosen={entry.id === mode} pick={() => change({ ...value, default_mode: entry.id })} />)}
    {!moreModes && front.length && behind.length ? <p><button type="button" className="btn" onClick={() => setMoreModes(true)}>More ways it can work</button></p> : null}
    {takesRules ? <>
      <p className="rules-summary">{summary(value)} {open ? null : <button type="button" className="btn" onClick={() => setOpen(true)}>Show every rule</button>}</p>
      {open ? <div className="rules">
        {LISTS.map((list) => {
          const rules = value[list.id] ?? [];
          const closed = kept && list.id === 'allow';
          return <section key={list.id} className="rule-list" aria-label={list.name}>
            <h4>{list.name}</h4>
            {closed ? <p className="dim">Not available with Kept to its folder: that setting cannot carry rules that run without asking.</p> : null}
            <ul>
              {rules.map((rule) => <li key={rule} className="rule-row">{wordsFor(rule)} <small><code>{rule}</code></small> <button type="button" className="btn" onClick={() => remove(list.id, rule)}>Remove</button></li>)}
              {list.id === 'deny' && agent ? <Forced agent={agent} /> : null}
            </ul>
            {!rules.length && !closed && !(list.id === 'deny' && agent) ? <p className="dim">Nothing.</p> : null}
          </section>;
        })}
        {adding ? <AddRule lists={kept ? ['ask', 'deny'] : ['allow', 'ask', 'deny']} computers={computers} computer={computer} add={add} done={() => setAdding(false)} />
          : <p><button type="button" className="btn" onClick={() => setAdding(true)}>Add a rule</button></p>}
        {extraFolders}
      </div> : null}
    </> : <>
      <p className="dim">{program.name} takes no rules about single tools or files.</p>
      {extraFolders}
    </>}
  </div>;
}
