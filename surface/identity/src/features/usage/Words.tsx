/** An agent's words (AGENTS-001 R5): each of the five slots as it resolves for the agent, where it comes from, the agent's own layer with a form to set it from the revision read, and a preview that renders a slot now and sends nothing. */
import { useState } from 'react';
import { agentWords } from '../../api';
import { ErrorWords, IdentityName } from '../people/Words';
import { clock } from '../file/time';
import { useRoleChange } from '../roles/useRoleChange';
import { ChangeStatus } from '../roles/ChangeStatus';
import { Act } from '../../shell/Act';
import { SLOTS } from './contract';
import type { WordsDelivered, WordsForAgent, WordsHeld, WordsResolved, WordsSetting, WordsSlot } from './contract';

type Props = { agent: string; words: WordsForAgent; changed: (words: string) => void };

const COLUMNS = ['18%', '34%', '12%', '36%'];

const SOURCES: Record<string, string> = { session: 'This session', agent: 'Set for this agent', workspace: 'The workspace', built_in: 'Lys’s own words', profile: 'The agent’s program' };

export function Words({ agent, words, changed }: Props) {
  return <section className="card usage-words" aria-label="Messages it is sent"><h3>Messages it is sent</h3>
    <p className="note">What Lys sends this agent at each moment. Choose Its own words to change one for this agent only.</p>
    <table className="usage-list usage-table">
      <colgroup>{COLUMNS.map((width, index) => <col key={index} style={{ width }} />)}</colgroup>
      <thead><tr><th>Message</th><th>What the agent is sent</th><th>Comes from</th><th>Set for this agent</th></tr></thead>
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
    <td>{SOURCES[resolved.source] ?? 'Elsewhere'}</td>
    <td>
      <form id={form} aria-label={'Set the ' + SLOTS[slot].toLowerCase() + ' words'} onSubmit={(event) => {
        event.preventDefault(); const next = setting(); if (!save.blocked && next) save.submit({ setting: next, revision });
      }}>
        <select aria-label={'What this agent is sent for ' + SLOTS[slot].toLowerCase()} value={kind} disabled={save.blocked} onChange={(event) => setKind(event.target.value as WordsSetting['kind'])}>
          <option value="inherit">The words shown, as they come</option>
          <option value="text">Its own words</option>
          <option value="template">A named template</option>
        </select>
        {kind === 'text' ? <textarea name="text" className="usage-grow" rows={2} aria-label={'Own words for ' + SLOTS[slot].toLowerCase()} placeholder="What this agent is sent instead" value={text} required disabled={save.blocked} onChange={(event) => setText(event.target.value)} /> : null}
        {kind === 'template' ? <input name="template" aria-label={'Template for ' + SLOTS[slot].toLowerCase()} placeholder="The template’s name, for example night-shift" value={name} required disabled={save.blocked} onChange={(event) => setName(event.target.value)} /> : null}
        {kind === 'template' && name !== '' && setting() === null ? <p className="why-not">A template’s name is lower-case letters, digits, - and _ only.</p> : null}
        <Act symbol="approve" type="submit" name={'Save the ' + SLOTS[slot].toLowerCase() + ' words'} word="Save" disabled={save.blocked || setting() === null} />
        <div className="note">{held ? <>Set for this agent by <IdentityName id={held.by} />, {clock(held.at)}</> : 'Nothing set for this agent: it is sent the words shown.'}</div>
        <ChangeStatus change={save} />
      </form>
    </td>
  </tr>;
}

/** The slot as it resolves, and on request as it renders now for this agent with a sample context figure; nothing is sent. */
function Preview({ agent, slot, resolved }: { agent: string; slot: WordsSlot; resolved: WordsResolved }) {
  const [shown, setShown] = useState<WordsDelivered | null>(null);
  const [failure, setFailure] = useState<unknown>(null);
  const [busy, setBusy] = useState(false);
  const preview = async () => {
    if (busy) return;
    setBusy(true); setFailure(null);
    try { setShown(await agentWords.preview({ slot, agent, numbers: { context_percent: '20', context_figures: '20% of your context window (200,000 of 1,000,000 tokens)', compaction: 'compaction is at 40% (400,000 tokens)', compaction_figures: '40% (400,000 tokens)', message: 'A sample message.', text: 'A sample reminder.' } })); }
    catch (error) { setFailure(error); }
    finally { setBusy(false); }
  };
  return <div className="words-preview">
    <p className="words-template">{resolved.text ?? 'Not set: the profile names the command.'}</p>
    {resolved.text ? <Act symbol="again" name={'Preview the ' + SLOTS[slot].toLowerCase() + ' words'} word="Preview" hint="Show the words as this agent would be sent them now; nothing is sent" disabled={busy} onClick={() => void preview()} /> : null}
    {shown ? <p role="status" aria-label={'Preview of the ' + SLOTS[slot].toLowerCase() + ' words'}>{shown.text}{shown.missing.length ? ' (not filled in: ' + shown.missing.join(', ') + ')' : ''}</p> : null}
    {failure ? <><p className="why-not">The preview could not be shown.</p><ErrorWords problem={failure} /></> : null}
  </div>;
}
