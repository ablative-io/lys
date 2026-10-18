/**
 * The way out of a kept change this tab can no longer read back. A form keeps each change before sending it, so an
 * uncertain answer is checked rather than sent twice; when the kept copy cannot be read, sending pauses. Where an
 * operation id is still legible in what is left of the copy, this asks Lys itself, `GET /operations/{operation}`,
 * which answers for the signed-in person's own changes from any device: recorded, with what it made and a link to it;
 * not recorded, so it did not go through and the form opens again with nothing typed lost; or a refusal, and then Lys
 * cannot tell. A change with no operation of its own (a decline) is asked by what it was about. With nothing legible to
 * ask about the words say at once that Lys cannot tell, say not to do it again, and name where to look. Only the
 * person's own press clears the copy.
 */
import { useState } from 'react';
import type { ReactNode } from 'react';
import { Refused, request } from '../../api';
import { releaseRecord } from '../../kept';
import { failureWords } from '../signin/words';
import { Act } from '../../shell/Act';
import './grants.css';

/** `GET /operations/{operation}`'s answer: what Lys made under an operation the person sent, or that it has none. */
export type OperationAnswer = { recorded: true; kind: 'grant' | 'request' | 'decision' | 'role_change'; id: string } | { recorded: false };

/** Where what Lys recorded can be seen. */
const placeOf = (answer: Extract<OperationAnswer, { recorded: true }>): string =>
  answer.kind === 'role_change' ? '#/roles/' + encodeURIComponent(answer.id)
    : answer.kind === 'grant' ? '#/access'
      : '#/requests/' + encodeURIComponent(answer.id);

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
 * What a list says of a change with no operation of its own: `true`, it has it; `false`, it does not, read from a
 * list that would hold it; `null`, what was read could not hold it, so Lys cannot tell.
 */
export type Recorded = boolean | null;

type Answer = { at: 'asking' } | { at: 'checking' } | { at: 'found'; href: string | null } | { at: 'absent' } | { at: 'unknown' } | { at: 'failed'; words: string };

/** Ask Lys about each operation: the first it recorded, else that it recorded none; a refusal is Lys not saying. */
async function askLys(operations: string[]): Promise<Answer> {
  for (const operation of operations) {
    let answer: OperationAnswer;
    try {
      answer = await request<OperationAnswer>('/operations/' + encodeURIComponent(operation));
    } catch (error) {
      // A refusal to say (another person's operation, a caller not admitted) is Lys not telling; Lys not answering is a failure to check again.
      if (error instanceof Refused && error.status >= 400 && error.status < 500) return { at: 'unknown' };
      throw error;
    }
    if (answer.recorded) return { at: 'found', href: placeOf(answer) };
  }
  return { at: 'absent' };
}

export function StuckRecord({ keys, what, recorded, settled, byOperation = true, looked, again, look, where }: {
  /** The storage keys the form's kept copies live under. */
  keys: readonly string[];
  /** What was kept, as a person says it: "your request", "the access you gave", "your decision", "your job change". */
  what: string;
  /** For a change with no operation of its own: what Lys's lists say of it. */
  recorded?: (operations: string[]) => Promise<Recorded>;
  /** Called once the copies are released, so the form reads its storage again and opens. */
  settled: () => void;
  /** False where Lys is asked by what the change was about rather than by its operation, which a decline has none of. */
  byOperation?: boolean;
  /** The list a `false` was read from, for a change asked by what it was about: "the requests you may decide". */
  looked?: string;
  /** The act not to repeat while Lys cannot tell: "give it", "ask for it", "decide it", "make this change". */
  again: string;
  /** Where to look when Lys cannot tell, ending in a full stop. */
  look: ReactNode;
  /** Where a change Lys has can be seen, when that is not the page it was made on. */
  where?: ReactNode;
}) {
  const askable = byOperation ? legibleOperations(keys).length > 0 : recorded !== undefined;
  const [answer, setAnswer] = useState<Answer>(askable ? { at: 'asking' } : { at: 'unknown' });
  const release = () => { for (const key of keys) releaseRecord(key); settled(); };
  const check = async () => {
    setAnswer({ at: 'checking' });
    try {
      if (byOperation) { setAnswer(await askLys(legibleOperations(keys))); return; }
      const said = recorded ? await recorded([]) : null;
      setAnswer(said === null ? { at: 'unknown' } : said ? { at: 'found', href: null } : { at: 'absent' });
    } catch (error) {
      setAnswer({ at: 'failed', words: failureWords(error, 'Nothing was cleared. Check again when Lys is answering.') });
    }
  };
  return <div className="stuck-record" role="status">
    <p>Lys could not read back {what} from earlier in this tab, so sending is paused: sending again might do it twice.</p>
    {answer.at === 'asking' || answer.at === 'checking' || answer.at === 'failed'
      ? <Act symbol="again" name={answer.at === 'checking' ? 'Checking with Lys…' : 'Check with Lys'} word={answer.at === 'checking' ? 'Checking…' : 'Check with Lys'} disabled={answer.at === 'checking'} onClick={() => void check()} /> : null}
    {answer.at === 'failed' ? <p className="why-not" role="alert">{answer.words}</p> : null}
    {answer.at === 'found' ? <><p>{byOperation ? <>Lys recorded this: {what} went through</> : <>Lys has {what}: it went through</>}, so there is nothing to send again.
      {where ? <> You can see it on {where}.</> : answer.href ? <> <a href={answer.href}>See it</a>.</> : null}</p>
      <Act symbol="approve" name="Carry on" word="Carry on" tone="primary" onClick={release} /></> : null}
    {answer.at === 'absent' ? <>
      <p>{byOperation
        ? <>Lys has no record of {what}, so it did not go through. Nothing you typed is lost: clear it and the form opens again.</>
        : <>Lys has no record of it among {looked ?? 'what it showed you'}, which would show it, so it did not go through. Clear it to carry on.</>}</p>
      <Act symbol="close" name="Clear it and carry on" word="Clear it" onClick={release} />
    </> : null}
    {answer.at === 'unknown' ? <>
      <p>Lys cannot tell from what you may see whether this went through. Do not {again} again. {look}</p>
      <Act symbol="close" name="Forget this kept copy" word="Forget it" onClick={release} />
    </> : null}
  </div>;
}
