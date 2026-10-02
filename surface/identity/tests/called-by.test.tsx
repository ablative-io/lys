import { describe, expect, it } from 'vitest';
import { calledBy } from '../src/features/people/directory';
import { ADA, SCRIBE } from './fixtures';

describe('A sentence names someone as people would', () => {
  it('names an agent by its whole name, never its first word', () => {
    expect(calledBy(SCRIBE, 'Walk Helper')).toBe('Walk Helper');
  });

  it('names a person by their first name', () => {
    expect(calledBy(ADA, 'Ada Lovelace')).toBe('Ada');
  });
});
