/**
 * The way out of a kept change this tab can no longer read back. A form keeps each change before sending it, so an
 * uncertain answer is checked rather than sent twice; when the kept copy cannot be read, sending pauses. This asks
 * Lys whether it has the operations named in what is left of the copy. Lys having one settles it; Lys having none of
 * them, or no operation being legible, is said in those words, and only the person's own press clears the copy.
 */
import { useState } from 'react';
import { releaseRecord } from '../../kept';
import { failureWords } from '../signin/words';
import { Act } from '../../shell/Act';
import './grants.css';

/** Every operation id still legible in the kept copies under `keys`, however damaged the rest of each copy is. */
export function legibleOperations(keys: readonly string[]): string[] {
  const found = keys.flatMap((key) => (sessionStorage.getItem(key) ?? '').match(/op-[0-9a-f]{32}/g) ?? []);
  return [...new Set(found)];
}

type Answer = { at: 'asking' } | { at: 'checking' } | { at: 'found' } | { at: 'absent'; legible: boolean } | { at: 'failed'; words: string };

export function StuckRecord({ keys, what, recorded, settled, byOperation = true }: {
  /** The storage keys the form's kept copies live under. */
  keys: readonly string[];
  /** What was kept, as a person says it: "your request", "the access you gave", "your decision", "your job change". */
  what: string;
  /** Whether Lys has any of these operations; left out where Lys cannot be asked from here. */
  recorded?: (operations: string[]) => Promise<boolean>;
  /** Called once the copies are released, so the form reads its storage again and opens. */
  settled: () => void;
  /** False where Lys is asked by what the change was about rather than by its operation, which a decline has none of. */
  byOperation?: boolean;
}) {
  const [answer, setAnswer] = useState<Answer>(recorded ? { at: 'asking' } : { at: 'absent', legible: false });
  const release = () => { for (const key of keys) releaseRecord(key); settled(); };
  const check = async () => {
    if (!recorded) return;
    const operations = legibleOperations(keys);
    if (byOperation && !operations.length) { setAnswer({ at: 'absent', legible: false }); return; }
    setAnswer({ at: 'checking' });
    try {
      setAnswer(await recorded(operations) ? { at: 'found' } : { at: 'absent', legible: true });
    } catch (error) {
      setAnswer({ at: 'failed', words: failureWords(error, 'Nothing was cleared. Check again when Lys is answering.') });
    }
  };
  return <div className="stuck-record" role="status">
    <p>Lys could not read back {what} from earlier in this tab, so sending is paused: sending again might do it twice.</p>
    {answer.at === 'asking' || answer.at === 'checking' || answer.at === 'failed'
      ? <Act symbol="again" name={answer.at === 'checking' ? 'Checking with Lys…' : 'Check with Lys'} word={answer.at === 'checking' ? 'Checking…' : 'Check with Lys'} disabled={answer.at === 'checking'} onClick={() => void check()} /> : null}
    {answer.at === 'failed' ? <p className="why-not" role="alert">{answer.words}</p> : null}
    {answer.at === 'found' ? <><p>Lys has {what}: it went through, so there is nothing to send again.</p>
      <Act symbol="approve" name="Carry on" word="Carry on" tone="primary" onClick={release} /></> : null}
    {answer.at === 'absent' ? <>
      <p>{answer.legible ? `Lys has no record of it among what you can see, so it most likely did not go through.`
        : recorded ? 'Too little of it is left to ask Lys about.' : 'Lys cannot be asked about this one from here; look at the list on this page to see whether it went through.'}
        {' '}Clear it to carry on. If it did go through after all, it shows in your list and can be taken back there.</p>
      <Act symbol="close" name="Clear it and carry on" word="Clear it" onClick={release} />
    </> : null}
  </div>;
}
