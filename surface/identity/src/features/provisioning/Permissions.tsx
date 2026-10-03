/**
 * What an agent may do, on the settings form and the add form. It edits what
 * Lys already stores and writes into the start: the mode, the three rule
 * lists and the extra folders of the agent's settings.
 *
 * Everything is in the open, as tables. The ways the chosen program can work
 * are one table, each with its plain name and what it means, one chosen.
 * The rules are one table: what the rule covers in plain words, the exact
 * rule, what happens when the agent tries it (changed in the row), who
 * enforces it, and Remove; its last row adds one by choosing its kind and
 * giving the one thing that kind needs. The program's own grammar is never
 * typed. The rules the agent's policy forces are rows of the same table,
 * said to be the policy's. Extra folders are a table with an add row.
 * Nothing here says a rule is enforced.
 */
import { useState } from 'react';
import { request, useLoad } from '../../api';
import type { PolicyView } from '../file/policyContract';
import type { Program } from './choices';
import { FolderChooser } from './FolderChooser';
import { FIRST, LISTS, MODES, forcedBy } from './permission-modes';
import type { Forced, RuleList } from './permission-modes';
import { RULE_KINDS, ruleFor, summary, wordsFor } from './permission-rules';
import type { RuleKind } from './permission-rules';
import type { Permissions as Value } from './Provisioning';

type Computers = { id: string; name: string }[];

/** The mode Lys makes itself, which cannot carry rules under Without asking or extra folders (crates/lys-home/src/harness/claude_code/launch_env.rs, lines 75 to 86). */
const KEPT = 'workspace-only';

function Mode({ entry, chosen, pick }: { entry: Program['modes'][number]; chosen: boolean; pick: () => void }) {
  const known = MODES[entry.id];
  return <tr className="mode-row">
    <td><label className="tick"><input type="radio" name="permission-mode" value={entry.id} checked={chosen} onChange={pick} /><span><b>{known?.name ?? entry.id}</b>{known ? <small className="mode-id">{' ' + entry.id}</small> : null}{known?.warn ? <span className="mode-warn" role="img" aria-label="Warning"> ⚠</span> : null}</span></label></td>
    <td className="mode-meaning">{entry.meaning}</td>
  </tr>;
}

/** What was read of the agent's policy: nothing while an agent is being added, else the read as it stands. */
type Policy = { status: 'none' } | { status: 'loading' } | { status: 'unread'; reason: string; refusal: string } | { status: 'read'; forced: Forced };

/** The last row of the rules table: the kind, the one thing that kind needs, what happens, and Add. */
function AddRule({ lists, computers, computer, enforcer, add }: {
  lists: RuleList[]; computers: Computers; computer: string; enforcer: string; add: (list: RuleList, rule: string) => void;
}) {
  const [kind, setKind] = useState<RuleKind | ''>('');
  const [given, setGiven] = useState('');
  const [list, setList] = useState<RuleList | ''>('');
  const built = kind ? ruleFor(kind, given) : null;
  const rule = built && 'rule' in built ? built.rule : null;
  const problem = built && 'problem' in built ? built.problem : '';
  const folder = kind === 'read-folder' || kind === 'change-folder';
  const noEnter = (event: { key: string; preventDefault: () => void }) => { if (event.key === 'Enter') event.preventDefault(); };
  const typed = kind === 'command' ? { name: 'rule-command', label: 'The words the command starts with', hint: 'For example: git status' }
    : kind === 'website' ? { name: 'rule-host', label: 'The website’s name', hint: 'For example: example.org' }
    : kind === 'tool' ? { name: 'rule-tool', label: 'The tool’s name, exactly as the program names it', hint: 'For example: WebSearch' } : null;
  return <tr className="add-rule" role="group" aria-label="Add a rule">
    <td><select name="rule-kind" aria-label="What the rule is about" value={kind} onChange={(event) => { setKind(event.target.value as RuleKind | ''); setGiven(''); }}>
      <option value="">Choose what the rule is about</option>
      {RULE_KINDS.map(([id, words]) => <option key={id} value={id}>{words}</option>)}
    </select></td>
    <td>
      {folder ? <>
        {given ? <p className="works-in">The folder <code>{given}</code></p> : null}
        <FolderChooser computers={computers} preferred={computer} chosen={given} choose={setGiven} label={given ? 'Choose a different folder for this rule' : 'Choose the folder for this rule'} confirm="Use" />
      </> : null}
      {typed ? <input name={typed.name} aria-label={typed.label} placeholder={typed.hint} value={given} onChange={(event) => setGiven(event.target.value)} onKeyDown={noEnter} /> : null}
      {rule ? <p className="rule-row">{wordsFor(rule)} <small><code>{rule}</code></small></p> : null}
      {kind && given.trim() && problem ? <p className="why-not">{problem}</p> : null}
    </td>
    <td><select name="rule-list" aria-label="What happens when it tries this" value={list} onChange={(event) => setList(event.target.value as RuleList | '')}>
      <option value="">Choose what happens</option>
      {LISTS.filter((entry) => lists.includes(entry.id)).map((entry) => <option key={entry.id} value={entry.id}>{entry.name}</option>)}
    </select></td>
    <td className="dim">{enforcer}</td>
    <td><button type="button" className="btn primary" disabled={!rule || !list} onClick={() => { if (rule && list) { add(list, rule); setKind(''); setGiven(''); setList(''); } }}>Add this rule</button></td>
  </tr>;
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

const RULE_COLUMNS = ['30%', '30%', '18%', '12%', '10%'];

function Editor({ agent, program, value, change, computers, computer, tools = [], policy }: Props & { policy: Policy }) {
  const mode = value.default_mode ?? '';
  const modes = program?.modes ?? [];
  if (!program) return null;
  // The modes people reach for first lead the table; the rest follow in the catalogue's order. None is hidden.
  const ordered = [...FIRST.flatMap((id) => modes.filter((entry) => entry.id === id)), ...modes.filter((entry) => !FIRST.includes(entry.id))];
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
  const move = (from: RuleList, to: RuleList, rule: string) => {
    if (from === to) return;
    change({ ...value, [from]: (value[from] ?? []).filter((one) => one !== rule), [to]: (value[to] ?? []).includes(rule) ? value[to] : [...(value[to] ?? []), rule] });
  };
  const offered: RuleList[] = kept ? ['ask', 'deny'] : ['allow', 'ask', 'deny'];
  const folders = value.additional_directories ?? [];
  const own = LISTS.flatMap((list) => (value[list.id] ?? []).map((rule) => ({ list: list.id, rule })));
  const extraFolders = <table className="usage-table" aria-label="Extra folders">
    <thead><tr><th>Extra folders it may also work in</th><th>Change</th></tr></thead>
    <tbody>
      {folders.map((entry) => <tr key={entry} className="rule-row"><td><code>{entry}</code></td><td><button type="button" className="btn" aria-label={'Remove ' + entry} onClick={() => remove('additional_directories', entry)}>Remove</button></td></tr>)}
      {folders.length ? null : <tr><td colSpan={2} className="dim">{kept ? 'Not available with Kept to its folder: that setting keeps the agent to its one working folder.' : 'None. It works in its working folder only.'}</td></tr>}
      {kept && folders.length ? <tr><td colSpan={2} className="dim">Not available with Kept to its folder: that setting keeps the agent to its one working folder. Remove these before it can start as Kept to its folder.</td></tr> : null}
    </tbody>
    {kept ? null : <tfoot><tr><td colSpan={2}><FolderChooser computers={computers} preferred={computer} chosen="" choose={(folder) => add('additional_directories', folder)} label="Add an extra folder" confirm="Add" /></td></tr></tfoot>}
  </table>;
  return <div className="permissions wide" role="group" aria-label="What this agent may do">
    <table className="usage-table" aria-label="How it works">
      <colgroup><col style={{ width: '34%' }} /><col style={{ width: '66%' }} /></colgroup>
      <thead><tr><th>How {program.name} works</th><th>What that means</th></tr></thead>
      <tbody>{ordered.map((entry) => <Mode key={entry.id} entry={entry} chosen={entry.id === mode} pick={() => change({ ...value, default_mode: entry.id })} />)}</tbody>
    </table>
    {takesRules ? <>
      <p className="rules-summary">{summary(value)}{forced?.written.length ? ' This agent’s policy refuses ' + forced.written.length + (forced.written.length === 1 ? ' thing.' : ' things.') : ''} {unread}</p>
      {forced?.unwritable.map((id) => <p key={id} role="alert" className="why-not">This policy rule cannot be written for {program.name}, so the agent will not start until it is changed: {id}</p>)}
      {keptRules || keptFolders ? <p role="alert" className="why-not">Kept to its folder cannot carry {[keptRules ? 'rules that run without asking' : '', keptFolders ? 'extra folders' : ''].filter(Boolean).join(' or ')}. Remove these before it can start as Kept to its folder.</p> : null}
      {keptTools ? <p role="alert" className="why-not">These settings list tools, so it cannot start as Kept to its folder.</p> : null}
      <table className="usage-table rules" aria-label="Rules">
        <colgroup>{RULE_COLUMNS.map((width, index) => <col key={index} style={{ width }} />)}</colgroup>
        <thead><tr><th>What it covers</th><th>Exact rule</th><th>What happens</th><th>Enforced by</th><th>Change</th></tr></thead>
        <tbody>
          {own.map(({ list, rule }) => <tr key={list + ' ' + rule} className="rule-row" data-list={list}>
            <td>{wordsFor(rule)}</td>
            <td><code>{rule}</code></td>
            <td><select aria-label={'What happens: ' + rule} value={list} onChange={(event) => move(list, event.target.value as RuleList, rule)}>
              {LISTS.filter((entry) => offered.includes(entry.id) || entry.id === list).map((entry) => <option key={entry.id} value={entry.id}>{entry.name}</option>)}
            </select>{kept && list === 'allow' ? <p className="why-not">Not available with Kept to its folder: that setting cannot carry rules that run without asking.</p> : null}</td>
            <td className="dim">{program.name}</td>
            <td><button type="button" className="btn" aria-label={'Remove ' + rule} onClick={() => remove(list, rule)}>Remove</button></td>
          </tr>)}
          {forced?.written.map((rule) => <tr key={'forced ' + rule} className="rule-row locked" data-list="deny">
            <td>{wordsFor(rule)}</td><td><code>{rule}</code></td><td>Refused</td><td className="dim">from this agent’s policy</td><td />
          </tr>)}
          {unread ? <tr className="rule-row"><td colSpan={5}>{unread}</td></tr> : null}
          {!own.length && !forced?.written.length && !unread ? <tr><td colSpan={5} className="dim">Nothing.</td></tr> : null}
        </tbody>
        <tfoot><AddRule lists={offered} computers={computers} computer={computer} enforcer={program.name} add={add} /></tfoot>
      </table>
    </> : <>
      <p className="dim">{program.name} takes no rules about single tools or files.</p>
      {forced?.hard && agent ? <p role="alert" className="why-not">This agent’s policy has {forced.hard} {forced.hard === 1 ? 'rule' : 'rules'}. {program.name} cannot carry them, so this agent will not start until they are removed from its policy. <a href={'#/file/' + encodeURIComponent(agent) + '/policy'}>Open its policy</a></p> : null}
      {unread ? <p className="dim">{unread}</p> : null}
    </>}
    {extraFolders}
  </div>;
}
