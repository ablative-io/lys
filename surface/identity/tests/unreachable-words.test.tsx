/** The line every page shows when Lys does not answer names which Lys, the likely cause and what to keep; never "ask the administrator" alone. */
import { expect, it } from 'vitest';
import { Refused } from '../src/api';
import { problemWords, refusalWords } from '../src/features/people/Words';

it('names the address the page is open at, the likely cause and what to keep', () => {
  const words = 'Lys at ' + location.host + ' is not answering; the computer it runs on may be off or asleep. Keep any change whose outcome is unknown.';
  expect(refusalWords('ServiceUnreachable')).toBe(words);
  expect(problemWords(new Refused(0, { refusal: 'ServiceUnreachable', reason: 'the identity service could not be reached: TypeError' }))).toContain(words);
  expect(words).not.toContain('administrator');
});
