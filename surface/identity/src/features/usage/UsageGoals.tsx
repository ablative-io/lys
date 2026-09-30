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
    <p className="usage-note">What this agent is for. It is reminded of every active goal while it works.</p>
    <ul className="usage-lines">{active.map((goal) => <GoalLine key={goal.id} goal={goal} act={act} />)}</ul>
    <AddGoal agent={agent} act={act} />
    {inactive.length ? <details className="usage-inactive"><summary>{inactive.length} switched off</summary>
      <ul className="usage-lines">{inactive.map((goal) => <GoalLine key={goal.id} goal={goal} act={act} />)}</ul>
    </details> : null}
    {failure ? <p role="alert" className="why-not">{failure}</p> : null}
  </section>;
}

type Act = (path: string, body: object, done: string) => Promise<void>;

function GoalLine({ goal, act }: { goal: Goal; act: Act }) {
  const [words, setWords] = useState(goal.words);
  const [busy, setBusy] = useState(false);
  const path = '/goals/' + encodeURIComponent(goal.id);
  const run = async (step: () => Promise<void>) => { setBusy(true); await step(); setBusy(false); };
  const reword = () => {
    const next = words.trim();
    if (next && next !== goal.words) void run(() => act(path + '/words', { words: next }, 'Goal reworded.'));
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
