/** An agent's words (AGENTS-001 R5): each of the five slots as it resolves for the agent, where it comes from, the agent's own layer with a form to set it from the revision read, and a preview that renders a slot now and sends nothing. */
import { useState } from 'react';
import { Refused, agentWords } from '../../api';
import { useRoleChange } from '../roles/useRoleChange';
import { ChangeStatus } from '../roles/ChangeStatus';
import { Act } from '../../shell/Act';
import { SLOTS } from './contract';
import type { WordsDelivered, WordsForAgent, WordsHeld, WordsResolved, WordsSetting, WordsSlot } from './contract';

type Props = { agent: string; words: WordsForAgent; changed: (words: string) => void };

const COLUMNS = ['18%', '34%', '12%', '36%'];

const SOURCES: Record<string, string> = { session: 'This session', agent: 'This agent', workspace: 'The workspace', built_in: 'Built in', profile: 'The profile' };

export function Words({ agent, words, changed }: Props) {
  return <section className="card usage-words" aria-label="Words"><h3>Words</h3>
    <table className="usage-list usage-table">
      <colgroup>{COLUMNS.map((width, index) => <col key={index} style={{ width }} />)}</colgroup>
      <thead><tr><th>Slot</th><th>What the agent is sent</th><th>From</th><th>This agent's layer</th></tr></thead>
      <tbody>
        {words.resolved.map((resolved) => <SlotRow key={resolved.slot} agent={agent} resolved={resolved} held={words.held[resolved.slot]} changed={changed} />)}
      </tbody>
    </table>
  </section>;
}

function SlotRow({ agent, resolved, held, changed }: { agent: string; resolved: WordsResolved; held?: WordsHeld; changed: (words: string) => void }) {
  const slot = resolved.slot;
  const key = 'lys.pending.words.' + agent + '.' + slot;
  const path = '/agents/' + encodeURIComponent(agent) + '/words/' + slot;
  const revision = held?.revision ?? 0;
  const initial = held?.setting ?? { kind: 'inherit' as const };
  const [kind, setKind] = useState<WordsSetting['kind']>(initial.kind);
  const [text, setText] = useState(initial.kind === 'text' ? initial.text : '');
  const [name, setName] = useState(initial.kind === 'template' ? initial.name : '');
  const save = useRoleChange<{ revision: number }>(key, path,
    (answer, body) => typeof answer?.revision === 'number' && answer.revision === Number(body.revision) + 1,
    () => changed(SLOTS[slot] + ' words saved.'));
  const setting = (): WordsSetting | null => {
    if (kind === 'inherit') return { kind: 'inherit' };
    if (kind === 'text') return text.trim() ? { kind: 'text', text: text.trim() } : null;
    return /^[a-z0-9_-]{1,64}$/.test(name) ? { kind: 'template', name } : null;
  };
  const form = 'words-' + slot;
  return <tr>
    <td>{SLOTS[slot]}</td>
    <td><Preview agent={agent} slot={slot} resolved={resolved} /></td>
    <td>{SOURCES[resolved.source] ?? resolved.source}</td>
    <td>
      <form id={form} aria-label={'Set the ' + SLOTS[slot].toLowerCase() + ' words'} onSubmit={(event) => {
        event.preventDefault(); const next = setting(); if (!save.blocked && next) save.submit({ setting: next, revision });
      }}>
        <select aria-label={'Layer kind for ' + slot} value={kind} disabled={save.blocked} onChange={(event) => setKind(event.target.value as WordsSetting['kind'])}>
          <option value="inherit">Inherit</option>
          <option value="text">Own text</option>
          <option value="template">A template</option>
        </select>
        {kind === 'text' ? <input name="text" aria-label={'Text for ' + slot} value={text} required disabled={save.blocked} onChange={(event) => setText(event.target.value)} /> : null}
        {kind === 'template' ? <input name="template" aria-label={'Template for ' + slot} value={name} required disabled={save.blocked} onChange={(event) => setName(event.target.value)} /> : null}
        <Act symbol="approve" type="submit" name={'Save the ' + slot + ' words'} word="Save" disabled={save.blocked || setting() === null} />
        <div className="note">Revision {revision}{held ? ', by ' + held.by : ', never set'}</div>
        <ChangeStatus change={save} />
      </form>
    </td>
  </tr>;
}

/** The slot as it resolves, and on request as it renders now for this agent with a sample context figure; nothing is sent. */
function Preview({ agent, slot, resolved }: { agent: string; slot: WordsSlot; resolved: WordsResolved }) {
  const [shown, setShown] = useState<WordsDelivered | null>(null);
  const [failure, setFailure] = useState('');
  const [busy, setBusy] = useState(false);
  const preview = async () => {
    if (busy) return;
    setBusy(true); setFailure('');
    try { setShown(await agentWords.preview({ slot, agent, numbers: { context_percent: '50', message: 'A sample message.', text: 'A sample reminder.' } })); }
    catch (error) { setFailure(error instanceof Refused ? error.refusal.refusal + ': ' + error.message : String(error)); }
    finally { setBusy(false); }
  };
  return <div className="words-preview">
    <p className="words-template">{resolved.text ?? 'Not set: the profile names the command.'}</p>
    {resolved.text ? <Act symbol="again" name={'Preview the ' + slot + ' words'} word="Preview" disabled={busy} onClick={() => void preview()} /> : null}
    {shown ? <p role="status" aria-label={'Preview of ' + slot}>{shown.text}{shown.missing.length ? ' (not set: ' + shown.missing.join(', ') + ')' : ''}</p> : null}
    {failure ? <p role="status">{failure}</p> : null}
  </div>;
}
