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
import type { Forced, RuleList } from './permission-modes';
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

/** What was read of the agent's policy: nothing while an agent is being added, else the read as it stands. */
type Policy = { status: 'none' } | { status: 'loading' } | { status: 'unread'; reason: string; refusal: string } | { status: 'read'; forced: Forced };

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

interface Props {
  /** The agent whose policy forces rules; none while an agent is being added. */
  agent?: string; program: Program | undefined; value: Value; change: (next: Value) => void; computers: Computers; computer: string;
  /** The tools the settings list; a start puts each on the Without asking list, which Kept to its folder cannot carry (launch_permissions.rs 137-140). */
  tools?: string[];
}

export function Permissions(props: Props) {
  return props.agent ? <OfAgent {...props} agent={props.agent} /> : <Editor {...props} policy={{ status: 'none' }} />;
}

/** The agent's policy is read when the editor is shown, so the closed line never says there are no rules while the policy refuses things. */
function OfAgent(props: Props & { agent: string }) {
  const load = useLoad(() => request<PolicyView>('/agents/' + encodeURIComponent(props.agent) + '/policy'), 'permissions-policy:' + props.agent);
  const policy: Policy = load.status === 'loading' ? { status: 'loading' }
    : load.status === 'refused' ? { status: 'unread', reason: load.refused.message, refusal: load.refused.refusal.refusal }
    : load.data.policy !== null && !Array.isArray(load.data.policy?.rules) ? { status: 'unread', reason: 'The service answered, but not with this agent’s policy.', refusal: 'PolicyUnreadable' }
    : { status: 'read', forced: forcedBy(load.data.policy?.rules ?? []) };
  return <Editor {...props} policy={policy} />;
}

function Editor({ agent, program, value, change, computers, computer, tools = [], policy }: Props & { policy: Policy }) {
  const mode = value.default_mode ?? '';
  const modes = program?.modes ?? [];
  const front = FIRST.flatMap((id) => modes.filter((entry) => entry.id === id));
  const behind = modes.filter((entry) => !FIRST.includes(entry.id));
  const [moreModes, setMoreModes] = useState(() => behind.some((entry) => entry.id === mode));
  const [open, setOpen] = useState(false);
  const [adding, setAdding] = useState(false);
  if (!program) return null;
  // A program listed with no description takes no rules here; the service refuses permissions for one (provisioning_api.rs, `settings`).
  const takesRules = (program.description?.permissions?.rule_forms?.length ?? 0) > 0;
  const kept = mode === KEPT;
  const forced = policy.status === 'read' ? policy.forced : null;
  const unread = policy.status === 'unread' ? <>Lys could not read this agent’s policy, so what it refuses is not shown. {policy.reason} <small className="refusal-name">{policy.refusal}</small></> : null;
  // What Kept to its folder cannot carry, each of which makes the start be refused.
  const keptRules = kept ? (value.allow ?? []).length : 0;
  const keptFolders = kept ? (value.additional_directories ?? []).length : 0;
  const keptTools = kept ? tools.length : 0;
  const remove = (list: RuleList | 'additional_directories', entry: string) => change({ ...value, [list]: (value[list] ?? []).filter((one) => one !== entry) });
  const add = (list: RuleList | 'additional_directories', entry: string) => { if (!(value[list] ?? []).includes(entry)) change({ ...value, [list]: [...(value[list] ?? []), entry] }); };
  const folders = value.additional_directories ?? [];
  const extraFolders = <section className="rule-list" aria-label="Extra folders">
    <h4>Extra folders</h4>
    {kept ? <p className="dim">Not available with Kept to its folder: that setting keeps the agent to its one working folder.{keptFolders ? ' Remove these before it can start as Kept to its folder.' : ''}</p> : null}
    {folders.length ? <ul>{folders.map((entry) => <li key={entry} className="rule-row"><code>{entry}</code> <button type="button" className="btn" onClick={() => remove('additional_directories', entry)}>Remove</button></li>)}</ul> : kept ? null : <p className="dim">None. It works in its working folder only.</p>}
    {kept ? null : <FolderChooser computers={computers} preferred={computer} chosen="" choose={(folder) => add('additional_directories', folder)} label="Add an extra folder" confirm="Add" />}
  </section>;
  return <div className="permissions" role="group" aria-label="What this agent may do">
    <p><b>What {program.name} may do</b></p>
    {(moreModes ? [...front, ...behind] : front.length ? front : behind).map((entry) => <Mode key={entry.id} entry={entry} chosen={entry.id === mode} pick={() => change({ ...value, default_mode: entry.id })} />)}
    {!moreModes && front.length && behind.length ? <p><button type="button" className="btn" onClick={() => setMoreModes(true)}>More ways it can work</button></p> : null}
    {takesRules ? <>
      <p className="rules-summary">{summary(value)}{forced?.written.length ? ' This agent’s policy refuses ' + forced.written.length + (forced.written.length === 1 ? ' thing.' : ' things.') : ''} {unread} {open ? null : <button type="button" className="btn" onClick={() => setOpen(true)}>Show every rule</button>}</p>
      {forced?.unwritable.map((id) => <p key={id} role="alert" className="why-not">This policy rule cannot be written for {program.name}, so the agent will not start until it is changed: {id}</p>)}
      {keptRules || keptFolders ? <p role="alert" className="why-not">Kept to its folder cannot carry {[keptRules ? 'rules that run without asking' : '', keptFolders ? 'extra folders' : ''].filter(Boolean).join(' or ')}. Remove these before it can start as Kept to its folder.</p> : null}
      {keptTools ? <p role="alert" className="why-not">These settings list tools, so it cannot start as Kept to its folder.</p> : null}
      {open ? <div className="rules">
        {LISTS.map((list) => {
          const rules = value[list.id] ?? [];
          const closed = kept && list.id === 'allow';
          return <section key={list.id} className="rule-list" aria-label={list.name}>
            <h4>{list.name}</h4>
            {closed ? <p className="dim">Not available with Kept to its folder: that setting cannot carry rules that run without asking.{rules.length ? ' Remove these before it can start as Kept to its folder.' : ''}</p> : null}
            <ul>
              {rules.map((rule) => <li key={rule} className="rule-row">{wordsFor(rule)} <small><code>{rule}</code></small> <button type="button" className="btn" onClick={() => remove(list.id, rule)}>Remove</button></li>)}
              {list.id === 'deny' ? forced?.written.map((rule) => <li key={'forced ' + rule} className="rule-row locked">{wordsFor(rule)} <small><code>{rule}</code></small> <span className="dim">from this agent’s policy</span></li>) : null}
              {list.id === 'deny' && unread ? <li className="rule-row">{unread}</li> : null}
            </ul>
            {!rules.length && !closed && !(list.id === 'deny' && (forced?.written.length || unread)) ? <p className="dim">Nothing.</p> : null}
          </section>;
        })}
        {adding ? <AddRule lists={kept ? ['ask', 'deny'] : ['allow', 'ask', 'deny']} computers={computers} computer={computer} add={add} done={() => setAdding(false)} />
          : <p><button type="button" className="btn" onClick={() => setAdding(true)}>Add a rule</button></p>}
        {extraFolders}
      </div> : null}
    </> : <>
      <p className="dim">{program.name} takes no rules about single tools or files.</p>
      {forced?.hard && agent ? <p role="alert" className="why-not">This agent’s policy has {forced.hard} {forced.hard === 1 ? 'rule' : 'rules'}. {program.name} cannot carry them, so this agent will not start until they are removed from its policy. <a href={'#/file/' + encodeURIComponent(agent) + '/policy'}>Open its policy</a></p> : null}
      {unread ? <p className="dim">{unread}</p> : null}
      {extraFolders}
    </>}
  </div>;
}
