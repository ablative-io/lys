/** Memory screens distinguish absent homes, unreadable records and private provenance without leaking content. */
import { describe, expect, it } from 'vitest';
import { mount, text } from './harness';
import { ADA, SCRIBE, SERVICE, ok, refused } from './fixtures';
import type { MemoryAnswer } from '../src/features/file/AgentMemory';
const path = '/agents/' + SCRIBE + '/memory';
const answer: MemoryAnswer = { agent: SCRIBE, home: true, memories: [{ id: 'memory-one', session: 'session-one', point: 'checkpoint-three', lit_by: SCRIBE, lit_at: '2026-09-28T01:00:00Z', lit_in: null, epilogues: 2 }], skipped: [], last_given: { session: 'session-two', entry: 'entry-four', given_at: '2026-09-28T02:00:00Z', harness: 'test-harness', harness_version: '1.0.0', documents: 3, environment: 4 }, visible_to: { agent: SCRIBE, responsible: ADA, administrator: true }, notes_shown: false };
describe('Agent memory', () => {
  it('shows provenance and context delivery without rendering notes or transcripts', async () => {
    const { requests, posted } = await mount('#/file/' + SCRIBE + '/memory', { ...SERVICE, [path]: ok({ ...answer, note: 'PRIVATE-NOTE', transcript: 'PRIVATE-TRANSCRIPT' }) });
    expect(requests).toContain(path); expect(posted).toEqual([]);
    for (const value of ['memory-one', 'checkpoint-three', 'session-two', 'entry-four', 'test-harness', '1.0.0', 'responsible person']) expect(text()).toContain(value);
    expect(text()).not.toContain('PRIVATE-'); expect(text()).not.toContain('not built yet');
  });
  it('distinguishes an absent home from an existing empty home', async () => {
    await mount('#/file/' + SCRIBE + '/memory', { ...SERVICE, [path]: ok({ ...answer, home: false, memories: [], last_given: null }) });
    expect(text()).toContain('No home has been recorded'); expect(text()).not.toContain('No memories have been recorded');
  });
  it('names each skipped session instead of claiming a complete list', async () => {
    await mount('#/file/' + SCRIBE + '/memory', { ...SERVICE, [path]: ok({ ...answer, skipped: [{ session: 'damaged-session', reason: 'invalid receipt' }] }) });
    expect(text()).toContain('This list is incomplete'); expect(text()).toContain('damaged-session: invalid receipt');
  });
  it('preserves the server refusal rather than answering no home', async () => {
    await mount('#/file/' + SCRIBE + '/memory', { ...SERVICE, [path]: refused(503, 'MemoryUnavailable', 'No homes directory configured') });
    expect(text()).toContain('MemoryUnavailable'); expect(text()).not.toContain('No home has been recorded');
  });
  it('rejects another agent’s record', async () => {
    await mount('#/file/' + SCRIBE + '/memory', { ...SERVICE, [path]: ok({ ...answer, agent: ADA }) });
    expect(text()).toContain('did not name this agent'); expect(text()).not.toContain('memory-one');
  });
});
