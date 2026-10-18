/**
 * The way out of a kept change this tab can no longer read back. A form keeps each change before sending it, so an
 * uncertain answer is checked rather than sent twice; when the kept copy cannot be read, sending pauses. This asks
 * Lys whether it has the operations named in what is left of the copy. Lys having one settles it. "Did not go
 * through" is said only after reading a list that would hold the change; when no such list was read (nothing legible
 * to ask about, a change Lys cannot be asked about from here, a list the change may have left), the words say Lys
 * cannot tell, say not to do it again, and name where to look. Only the person's own press clears the copy.
 * The proper answer is the server's GET /operations/{operation} (CONSENSUS, Server asks, 03 item 6); this is the floor.
 */
import { useState } from 'react';
import type { ReactNode } from 'react';
import { releaseRecord } from '../../kept';
import { failureWords } from '../signin/words';
import { Act } from '../../shell/Act';
import './grants.css';

/** Every operation id still legible in the kept copies under `keys`, however damaged the rest of each copy is. */
export function legibleOperations(keys: readonly string[]): string[] {
  const found = keys.flatMap((key) => (sessionStorage.getItem(key) ?? '').match(/op-[0-9a-f]{32}/g) ?? []);
  return [...new Set(found)];
}

/** Every recipient still legible in the kept copies under `keys`. */
export function legibleRecipients(keys: readonly string[]): string[] {
  const found = keys.flatMap((key) => [...(sessionStorage.getItem(key) ?? '').matchAll(/"recipient"\s*:\s*"([^"]+)"/g)].map((match) => match[1]));
  return [...new Set(found)];
}

/**
 * What Lys's lists say of the kept change: `true`, it has it; `false`, it does not, read from a list that would hold
 * it; `null`, what was read could not hold it, so Lys cannot tell.
 */
export type Recorded = boolean | null;

type Answer = { at: 'asking' } | { at: 'checking' } | { at: 'found' } | { at: 'absent' } | { at: 'unknown' } | { at: 'failed'; words: string };

export function StuckRecord({ keys, what, recorded, settled, byOperation = true, looked, again, look, where }: {
  /** The storage keys the form's kept copies live under. */
  keys: readonly string[];
  /** What was kept, as a person says it: "your request", "the access you gave", "your decision", "your job change". */
  what: string;
  /** What Lys's lists say of these operations; left out where Lys cannot be asked from here. */
  recorded?: (operations: string[]) => Promise<Recorded>;
  /** Called once the copies are released, so the form reads its storage again and opens. */
  settled: () => void;
  /** False where Lys is asked by what the change was about rather than by its operation, which a decline has none of. */
  byOperation?: boolean;
  /** The list a `false` was read from, as a person says it: "your requests", "the access you have given". */
  looked?: string;
  /** The act not to repeat while Lys cannot tell: "give it", "ask for it", "decide it", "make this change". */
  again: string;
  /** Where to look when Lys cannot tell, ending in a full stop. */
  look: ReactNode;
  /** Where a change Lys has can be seen, when that is not the page it was made on. */
  where?: ReactNode;
}) {
  const [answer, setAnswer] = useState<Answer>(recorded ? { at: 'asking' } : { at: 'unknown' });
  const release = () => { for (const key of keys) releaseRecord(key); settled(); };
  const check = async () => {
    if (!recorded) return;
    const operations = legibleOperations(keys);
    if (byOperation && !operations.length) { setAnswer({ at: 'unknown' }); return; }
    setAnswer({ at: 'checking' });
    try {
      const said = await recorded(operations);
      setAnswer(said === null ? { at: 'unknown' } : said ? { at: 'found' } : { at: 'absent' });
    } catch (error) {
      setAnswer({ at: 'failed', words: failureWords(error, 'Nothing was cleared. Check again when Lys is answering.') });
    }
  };
  return <div className="stuck-record" role="status">
    <p>Lys could not read back {what} from earlier in this tab, so sending is paused: sending again might do it twice.</p>
    {answer.at === 'asking' || answer.at === 'checking' || answer.at === 'failed'
      ? <Act symbol="again" name={answer.at === 'checking' ? 'Checking with Lys…' : 'Check with Lys'} word={answer.at === 'checking' ? 'Checking…' : 'Check with Lys'} disabled={answer.at === 'checking'} onClick={() => void check()} /> : null}
    {answer.at === 'failed' ? <p className="why-not" role="alert">{answer.words}</p> : null}
    {answer.at === 'found' ? <><p>Lys has {what}: it went through, so there is nothing to send again.{where ? <> You can see it on {where}.</> : null}</p>
      <Act symbol="approve" name="Carry on" word="Carry on" tone="primary" onClick={release} /></> : null}
    {answer.at === 'absent' ? <>
      <p>Lys has no record of it among {looked ?? 'what it showed you'}, which would show it, so it did not go through. Clear it to carry on.</p>
      <Act symbol="close" name="Clear it and carry on" word="Clear it" onClick={release} />
    </> : null}
    {answer.at === 'unknown' ? <>
      <p>Lys cannot tell from what you may see whether this went through. Do not {again} again. {look}</p>
      <Act symbol="close" name="Forget this kept copy" word="Forget it" onClick={release} />
    </> : null}
  </div>;
}
