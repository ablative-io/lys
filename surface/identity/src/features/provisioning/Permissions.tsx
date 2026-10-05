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
 * typed. The rules of the agent's policy, which Lys judges before the program
 * is asked, are rows of the same table, said to be the policy's, added in the
 * same last row and removed in their own; they are kept as a numbered version
 * by their own button, because they are saved apart from the settings. Extra
 * folders are a table with an add row. Nothing here says a rule is enforced.
 */
import { useState } from 'react';
import { Refused, request, useLoad } from '../../api';
import { KINDS } from '../file/policyContract';
import type { PolicyView, Rule as PolicyRule, RuleKind as PolicyKind } from '../file/policyContract';
import { ErrorWords } from '../people/Words';
import type { Program } from './choices';
import { FolderChooser } from './FolderChooser';
import { FIRST, LISTS, MODES, forcedBy } from './permission-modes';
import type { Forced, RuleList } from './permission-modes';
import { RULE_KINDS, ruleFor, summary, wordsFor } from './permission-rules';
import type { RuleKind } from './permission-rules';
import type { Permissions as Value } from './Provisioning';
import './permissions.css';
import { Act } from '../../shell/Act';

type Computers = { id: string; name: string }[];

/** The mode Lys makes itself, which cannot carry rules under Without asking or extra folders (crates/lys-home/src/harness/claude_code/launch_env.rs, lines 75 to 86). */
const KEPT = 'workspace-only';

function Mode({ entry, chosen, pick }: { entry: Program['modes'][number]; chosen: boolean; pick: () => void }) {
  const known = MODES[entry.id];
  const id = 'permission-mode-' + entry.id;
  // Three fixed columns: the radio, the name with the program's own name for it beneath, whole, and what it means.
  return <tr className="mode-row">
    <td className="mode-pick"><input type="radio" id={id} name="permission-mode" value={entry.id} checked={chosen} onChange={pick} /></td>
    <td className="mode-name"><label htmlFor={id}><b>{known?.name ?? entry.id}</b>{known?.warn ? <span className="mode-warn" role="img" aria-label="Warning"> ⚠</span> : null}</label>{known ? <small className="mode-id">{entry.id}</small> : null}</td>
    <td className="mode-meaning">{entry.meaning}</td>
  </tr>;
}

/** What was read of the agent's policy: nothing while an agent is being added, else the read as it stands. */
type Policy = { status: 'none' } | { status: 'loading' } | { status: 'unread'; reason: string; refusal: string } | { status: 'read'; forced: Forced; view: PolicyView; kept: (answer: PolicyView) => void };

/** Who can lift a policy rule. */
function lifts(rule: PolicyRule): string {
  if (rule.authority === 'hard') return 'Nobody: no grant can lift it';
  const { resource, action } = rule.authority.permission;
  return 'A grant of ' + action + ' on ' + resource.kind + ' ' + resource.id;
}

function said(failure: unknown): string {
  if (failure instanceof Refused && failure.refusal.refusal === 'PolicyVersionConflict') return 'PolicyVersionConflict: Someone else changed these rules.';
  if (failure instanceof Refused) return failure.refusal.reason;
  return String(failure);
}

/** The last row of the rules table: the kind, the one thing that kind needs, what happens, and Add. */
function AddRule({ lists, computers, computer, enforcer, add, addPolicy }: {
  lists: RuleList[]; computers: Computers; computer: string; enforcer: string; add: (list: RuleList, rule: string) => void;
  /** Adds a rule to the agent's policy, when the policy was read; without it the row adds the program's rules only. */
  addPolicy?: (rule: PolicyRule) => void;
}) {
  const [chosenBy, setBy] = useState<'program' | 'policy'>('program');
  // A program that takes no rules of its own leaves only the policy to add to.
  const by = lists.length ? chosenBy : 'policy';
  const blank = { id: '', tool: '', target: '', resource_kind: '', resource_id: '', action: '' };
  const [policy, setPolicy] = useState(blank);
  const [denies, setDenies] = useState<PolicyKind>('tool');
  const [grantable, setGrantable] = useState(false);
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
  const who = addPolicy ? <select name="rule-enforcer" aria-label="Who enforces the rule" value={by} onChange={(event) => setBy(event.target.value === 'policy' ? 'policy' : 'program')}>
    {lists.length ? <option value="program">{enforcer}</option> : null}
    <option value="policy">Lys, by this agent’s policy</option>
  </select> : <span className="dim">{enforcer}</span>;
  if (by === 'policy' && addPolicy) {
    const field = (name: keyof typeof blank, label: string) => <input name={name} aria-label={label} placeholder={label} value={policy[name]} onChange={(event) => setPolicy({ ...policy, [name]: event.target.value })} onKeyDown={noEnter} />;
    const value = (name: keyof typeof blank) => policy[name].trim();
    const ready = Boolean(value('id') && value('tool') && (denies === 'tool' || value('target')) && (!grantable || (value('resource_kind') && value('resource_id') && value('action'))));
    return <tr className="add-rule" role="group" aria-label="Add a rule">
      <td>{field('id', 'Rule name')}{field('tool', 'Tool name, exactly as the agent’s program names it')}</td>
      <td><select name="kind" aria-label="Denies" value={denies} onChange={(event) => setDenies(event.target.value as PolicyKind)}>
        {Object.entries(KINDS).map(([k, words]) => <option key={k} value={k}>{words}</option>)}
      </select>{denies !== 'tool' ? field('target', denies === 'host' ? 'Host' : 'Absolute path') : null}</td>
      <td>Refused</td>
      <td>{who}
        <label className="tick"><input type="checkbox" name="grantable" checked={grantable} onChange={(event) => setGrantable(event.target.checked)} />An access permission may allow this call</label>
        {grantable ? <>{field('resource_kind', 'Type of thing the permission covers')}{field('resource_id', 'Name of the thing the permission covers')}{field('action', 'Action')}</> : null}
      </td>
      <td><Act symbol="add" name="Add rule" word="Add" tone="primary" disabled={!ready} onClick={() => {
        if (!ready) return;
        const rule: PolicyRule = { id: value('id'), tool: value('tool'), kind: denies,
          authority: grantable ? { permission: { resource: { kind: value('resource_kind'), id: value('resource_id') }, action: value('action') } } : 'hard' };
        if (denies !== 'tool') rule.target = value('target');
        addPolicy(rule); setPolicy(blank); setDenies('tool'); setGrantable(false);
      }} /></td>
    </tr>;
  }
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
    <td>{who}</td>
    <td><Act symbol="add" name="Add this rule" word="Add" tone="primary" disabled={!rule || !list} onClick={() => { if (rule && list) { add(list, rule); setKind(''); setGiven(''); setList(''); } }} /></td>
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

/** The agent's policy is read when the editor is shown, so the table never says there are no rules while the policy refuses things. A kept version is shown from its own answer, without a second read. */
function OfAgent(props: Props & { agent: string }) {
  const [kept, setKept] = useState<PolicyView | null>(null);
  const load = useLoad(() => request<PolicyView>('/agents/' + encodeURIComponent(props.agent) + '/policy'), 'permissions-policy:' + props.agent);
  const view = kept ?? (load.status === 'ok' ? load.data : null);
  const policy: Policy = load.status === 'loading' ? { status: 'loading' }
    : load.status === 'refused' ? { status: 'unread', reason: load.refused.message, refusal: load.refused.refusal.refusal }
    : !view || view.agent !== props.agent || (view.policy !== null && !Array.isArray(view.policy?.rules)) ? { status: 'unread', reason: 'The service answered, but not with this agent’s policy.', refusal: 'PolicyUnreadable' }
    : { status: 'read', forced: forcedBy(view.policy?.rules ?? []), view, kept: setKept };
  return <Editor {...props} policy={policy} />;
}

const RULE_COLUMNS = ['30%', '30%', '18%', '12%', '10%'];

function Editor({ agent, program, value, change, computers, computer, tools = [], policy }: Props & { policy: Policy }) {
  const mode = value.default_mode ?? '';
  const modes = program?.modes ?? [];
  // The policy's rules as they will be kept: the saved ones until one is added or removed here, then the changed list until it is saved.
  const [staged, setStaged] = useState<PolicyRule[] | null>(null);
  const [saving, setSaving] = useState(false);
  const [failure, setFailure] = useState<string | null>(null);
  // With no program chosen there are no ways of working or program rules to show, but the agent’s policy is still its own and still shown.
  if (!program && policy.status === 'none') return null;
  const programName = program?.name ?? 'the program';
  // The modes people reach for first lead the table; the rest follow in the catalogue's order. None is hidden.
  const ordered = [...FIRST.flatMap((id) => modes.filter((entry) => entry.id === id)), ...modes.filter((entry) => !FIRST.includes(entry.id))];
  // A program listed with no description takes no rules here; the service refuses permissions for one (provisioning_api.rs, `settings`).
  const takesRules = (program?.description?.permissions?.rule_forms?.length ?? 0) > 0;
  const kept = mode === KEPT;
  const forced = policy.status === 'read' ? policy.forced : null;
  const read = policy.status === 'read' ? policy : null;
  const policyRules = staged ?? read?.view.policy?.rules ?? [];
  const keep = async () => {
    if (!read || !agent || !staged || saving) return;
    const version = read.view.policy?.version ?? 0;
    setSaving(true); setFailure(null);
    try {
      const answer = await request<PolicyView>('/agents/' + encodeURIComponent(agent) + '/policy', { version, rules: staged });
      if (answer.agent !== agent || answer.policy?.agent !== agent || answer.policy.version !== version + 1) throw new Error('The answer did not confirm the saved rules. Open this tab again to check the outcome.');
      read.kept(answer); setStaged(null);
    } catch (error) { setFailure(said(error)); }
    finally { setSaving(false); }
  };
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
      {folders.map((entry) => <tr key={entry} className="rule-row"><td><code>{entry}</code></td><td><Act symbol="remove" name={'Remove ' + entry} onClick={() => remove('additional_directories', entry)} /></td></tr>)}
      {folders.length ? null : <tr><td colSpan={2} className="dim">{kept ? 'Not available with Kept to its folder: that setting keeps the agent to its one working folder.' : 'None. It works in its working folder only.'}</td></tr>}
      {kept && folders.length ? <tr><td colSpan={2} className="dim">Not available with Kept to its folder: that setting keeps the agent to its one working folder. Remove these before it can start as Kept to its folder.</td></tr> : null}
    </tbody>
    {kept ? null : <tfoot><tr><td colSpan={2}><FolderChooser computers={computers} preferred={computer} chosen="" choose={(folder) => add('additional_directories', folder)} label="Add an extra folder" word="Add" confirm="Add" /></td></tr></tfoot>}
  </table>;
  return <div className="permissions wide" role="group" aria-label="What this agent may do">
    {program ? <table className="usage-table modes" aria-label="How it works">
      <colgroup><col className="mode-pick" /><col className="mode-name" /><col /></colgroup>
      <thead><tr><th aria-label="Chosen" /><th>How {programName} works</th><th>What that means</th></tr></thead>
      <tbody>{ordered.map((entry) => <Mode key={entry.id} entry={entry} chosen={entry.id === mode} pick={() => change({ ...value, default_mode: entry.id })} />)}</tbody>
    </table> : null}
    {takesRules ? <>
      <p className="rules-summary">{summary(value)}{forced?.written.length ? ' This agent’s policy refuses ' + forced.written.length + (forced.written.length === 1 ? ' thing.' : ' things.') : ''} {unread}</p>
      {forced?.unwritable.map((id) => <p key={id} role="alert" className="why-not">This policy rule cannot be written for {programName}, so the agent will not start until it is changed: {id}</p>)}
      {keptRules || keptFolders ? <p role="alert" className="why-not">Kept to its folder cannot carry {[keptRules ? 'rules that run without asking' : '', keptFolders ? 'extra folders' : ''].filter(Boolean).join(' or ')}. Remove these before it can start as Kept to its folder.</p> : null}
      {keptTools ? <p role="alert" className="why-not">These settings list tools, so it cannot start as Kept to its folder.</p> : null}
    </> : null}
    {takesRules || read ? <table className="usage-table rules" aria-label="Rules">
        <colgroup>{RULE_COLUMNS.map((width, index) => <col key={index} style={{ width }} />)}</colgroup>
        <thead><tr><th>What it covers</th><th>Exact rule</th><th>What happens</th><th>Enforced by</th><th>Change</th></tr></thead>
        <tbody>
          {(takesRules ? own : []).map(({ list, rule }) => <tr key={list + ' ' + rule} className="rule-row" data-list={list}>
            <td>{wordsFor(rule)}</td>
            <td><code>{rule}</code></td>
            <td><select aria-label={'What happens: ' + rule} value={list} onChange={(event) => move(list, event.target.value as RuleList, rule)}>
              {LISTS.filter((entry) => offered.includes(entry.id) || entry.id === list).map((entry) => <option key={entry.id} value={entry.id}>{entry.name}</option>)}
            </select>{kept && list === 'allow' ? <p className="why-not">Not available with Kept to its folder: that setting cannot carry rules that run without asking.</p> : null}</td>
            <td className="dim">{programName}</td>
            <td><Act symbol="remove" name={'Remove ' + rule} onClick={() => remove(list, rule)} /></td>
          </tr>)}
          {read ? policyRules.map((rule, index) => {
            const written = forcedBy([rule]);
            return <tr key={'policy ' + rule.id + ':' + index} className="rule-row locked" data-list="deny" data-policy={rule.id}>
              <td>{rule.tool}: {KINDS[rule.kind].toLowerCase()}{rule.target ? ', ' + rule.target : ''} <small className="mode-id">{rule.id}</small></td>
              <td>{rule.authority !== 'hard' ? <span className="dim">Judged by Lys at each call</span>
                : written.written.length ? written.written.map((each) => <span key={each}><code>{each}</code> </span>)
                : <span className="why-not">Cannot be written for {programName}</span>}</td>
              <td>Refused</td>
              <td className="dim">from this agent’s policy. {lifts(rule)}</td>
              <td><Act symbol="remove" name={'Remove policy rule ' + rule.id} disabled={saving} onClick={() => setStaged(policyRules.filter((_, at) => at !== index))} /></td>
            </tr>;
          }) : null}
          {unread ? <tr className="rule-row"><td colSpan={5}>{unread}</td></tr> : null}
          {!own.length && !policyRules.length && !unread ? <tr><td colSpan={5} className="dim">Nothing.</td></tr> : null}
          {read ? <tr className="policy-kept"><td colSpan={5}>
            <b>{read.view.policy ? 'Policy: Version ' + read.view.policy.version : 'No policy set'}.</b> A kept version {read.view.applies}. Sessions already running keep their original rules.
            {' '}
            <Act symbol="save" name="Save policy rules for the next start" word="Save" disabled={saving || staged === null} onClick={() => { void keep(); }} />
            {staged !== null && !saving ? <span className="dim"> The policy rows above are changed and not yet kept.</span> : null}
            {failure ? <ErrorWords problem={failure} /> : null}
          </td></tr> : null}
        </tbody>
        <tfoot><AddRule lists={takesRules ? offered : []} computers={computers} computer={computer} enforcer={programName} add={add} addPolicy={read ? (rule) => setStaged([...policyRules, rule]) : undefined} /></tfoot>
      </table> : null}
    {takesRules || !program ? null : <>
      <p className="dim">{programName} takes no rules about single tools or files.</p>
      {forced?.hard && agent ? <p role="alert" className="why-not">This agent’s policy has {forced.hard} {forced.hard === 1 ? 'rule' : 'rules'}. {programName} cannot carry them, so this agent will not start until they are removed from its policy, in the table above.</p> : null}
      {unread ? <p className="dim">{unread}</p> : null}
    </>}
    {program ? extraFolders : null}
  </div>;
}
