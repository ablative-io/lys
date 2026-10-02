/** An agent's goals: the sentences it is reminded of. A line is added with Enter, reworded in place, and switched off without being lost. */
import { useState } from 'react';
import { Refused, operationId, request } from '../../api';
import type { Goal } from './limits';

type Props = { agent: string; goals: Goal[]; changed: (words: string) => void };

/** Sends one change, answering the refusal in words or an empty string. */
async function send(path: string, body: object): Promise<string> {
  try {
    await request<unknown>(path, { operation: operationId(), ...body }, 'POST');
    return '';
  } catch (error) {
    return error instanceof Refused ? error.refusal.refusal + ': ' + error.message : String(error);
  }
}

export function UsageGoals({ agent, goals, changed }: Props) {
  const [failure, setFailure] = useState('');
  const act = async (path: string, body: object, done: string) => {
    const refused = await send(path, body);
    setFailure(refused);
    if (!refused) changed(done);
  };
  const active = goals.filter((goal) => goal.active);
  const inactive = goals.filter((goal) => !goal.active);
  return <section className="card usage-goals" aria-label="Goals"><h3>Goals</h3>
    {goals.length ? <table className="usage-list"><thead><tr><th>Kind</th><th>What</th><th>Deadline</th><th>Where it stands</th><th>Change</th></tr></thead>
      <tbody>{goals.map((item) => <tr key={item.goal.id}><td>{item.goal.kind}</td><td>{item.goal.words}</td>
        <td>{item.goal.deadline === null ? 'No deadline' : new Date(item.goal.deadline * 1000).toLocaleString()}</td><td>{STANDING[item.standing]}</td><td><GoalEdits agent={agent} item={item} changed={changed} /></td></tr>)}</tbody></table> : <p>No goal is set for this agent.</p>}
    <SetGoal agent={agent} changed={changed} />
  </section>;
}

function GoalEdits({ agent, item, changed }: { agent: string; item: GoalItem; changed: (words: string) => void }) {
  const goal = item.goal;
  const [words, setWords] = useState(goal.words);
  const key = 'lys.pending.goal.' + agent + '.' + goal.id;
  const path = '/goals/' + encodeURIComponent(goal.id);
  const reword = useRoleChange<GoalItem>(key + '.words', path + '/words',
    (answer, body) => answer?.goal?.id === goal.id && answer.goal.words === body.words,
    () => changed('Goal reworded.'));
  const activity = useRoleChange<GoalItem>(key + '.active', path + '/active',
    (answer, body) => answer?.goal?.id === goal.id && answer.goal.active === body.active,
    (answer) => changed(answer.goal.active ? 'Goal switched on.' : 'Goal switched off.'));
  const blocked = reword.blocked || activity.blocked || item.standing !== 'open';
  const next = words.trim();
  const valid = next.length > 0 && next.length <= 500 && !/\p{Cc}/u.test(next);
  return <div>
    <label>Active<input type="checkbox" aria-label={'Goal active ' + goal.id} checked={goal.active} disabled={blocked}
      onChange={(event) => { if (!blocked) activity.submit({ operation: operationId(), active: event.target.checked }); }} /></label>
    <ChangeStatus change={activity} />
    <form aria-label={'Reword goal ' + goal.id} onSubmit={(event) => {
      event.preventDefault(); if (!blocked && valid && next !== goal.words) reword.submit({ operation: operationId(), words: next });
    }}>
      <label className="field">Words<input name="words" value={words} required maxLength={500} disabled={blocked} onChange={(event) => setWords(event.target.value)} /></label>
      <button className="btn" type="submit" disabled={blocked || !valid || next === goal.words}>Reword goal</button>
      <ChangeStatus change={reword} />
    </form>
    {item.standing !== 'open' ? <p>Only open goals can be changed.</p> : null}
  </div>;
}

function SetGoal({ agent, changed }: { agent: string; changed: (words: string) => void }) {
  const [kind, setKind] = useState<GoalKind>('goal');
  const [words, setWords] = useState('');
  const [deadline, setDeadline] = useState('');
  const [evidence, setEvidence] = useState<'commit' | 'document' | 'check'>('commit');
  const path = '/agents/' + encodeURIComponent(agent) + '/goals';
  const change = useRoleChange<GoalItem>('lys.pending.goal.' + agent, path, (answer, body) => answer.goal.id === body.operation && answer.goal.words === body.words, () => changed('Goal kept.'));
  const submit = () => {
    const seconds = Math.floor(Date.parse(deadline) / 1000);
    if (!words.trim() || !Number.isFinite(seconds)) return;
    change.submit({ operation: operationId(), kind, words: words.trim(), deadline: seconds, ...(kind === 'deliverable' ? { evidence } : {}) });
  };
  return <li className="usage-goal" data-active={goal.active}>
    <label className="usage-switch"><input type="checkbox" aria-label={goal.active ? 'Switch this goal off' : 'Switch this goal on'} checked={goal.active} disabled={busy}
      onChange={() => void run(() => act(path + '/active', { active: !goal.active }, goal.active ? 'Goal switched off.' : 'Goal switched on.'))} /></label>
    <input className="usage-words" name="words" aria-label="Goal" value={words} maxLength={500} disabled={busy}
      onChange={(event) => setWords(event.target.value)} onBlur={reword} onKeyDown={(event) => { if (event.key === 'Enter') { event.preventDefault(); reword(); } }} />
    {goal.deadline === null ? null : <span className="usage-deadline">by {new Date(goal.deadline * 1000).toLocaleDateString()}</span>}
  </li>;
}

function AddGoal({ agent, act }: { agent: string; act: Act }) {
  const [words, setWords] = useState('');
  const [deadline, setDeadline] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const add = async () => {
    const next = words.trim();
    if (!next) return;
    const seconds = deadline ? Math.floor(Date.parse(deadline) / 1000) : null;
    setBusy(true);
    await act('/agents/' + encodeURIComponent(agent) + '/goals', { words: next, ...(seconds !== null && Number.isFinite(seconds) ? { deadline: seconds } : {}) }, 'Goal added.');
    setBusy(false);
    setWords(''); setDeadline(null);
  };
  return <form className="usage-add" aria-label="Add a goal" onSubmit={(event) => { event.preventDefault(); void add(); }}>
    <input className="usage-words" name="new-goal" aria-label="New goal" placeholder="Add a goal, then press Enter" value={words} maxLength={500} disabled={busy} onChange={(event) => setWords(event.target.value)} />
    {deadline === null
      ? <button className="btn" type="button" disabled={busy} onClick={() => setDeadline('')}>Add a deadline</button>
      : <input name="deadline" aria-label="Deadline" type="date" value={deadline} disabled={busy} onChange={(event) => setDeadline(event.target.value)} />}
  </form>;
}
