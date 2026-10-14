import { describe, expect, it } from 'vitest';
import { keepArrangement } from '../src/features/runtime/canvas-kept';
import { readArrangement } from '../src/features/runtime/canvas-marks';
import { ok, refused } from './fixtures';
import { serve } from './harness';

const operations = { view: 'canvas' as const, selected: 'seat-one', pane_width: 420 };
const arrangement = { boxes: {}, open: [], groups: [], notes: [], links: [], widgets: [], operations };

describe('the canvas carries the operations view with each arrangement', () => {
  it('retains the operations fields from a served arrangement', () => {
    expect(readArrangement(arrangement)).toEqual(arrangement);
  });

  it('keeps the arrangement in one PUT without a preceding read', async () => {
    const posted: { path: string; body: unknown }[] = [];
    const requests = serve({ '/canvas': refused(503, 'ReadUnavailable', 'a keep needs no read'), 'PUT /canvas': ok({ arrangement }) }, posted);
    await keepArrangement('service', arrangement);
    expect(requests).toEqual(['PUT /canvas']);
    expect(posted).toEqual([{ path: 'PUT /canvas', body: { arrangement } }]);
  });

  it('accepts the served width limits and refuses malformed operations', () => {
    for (const pane_width of [200, 640]) {
      expect(readArrangement({ ...arrangement, operations: { ...operations, pane_width } })?.operations?.pane_width).toBe(pane_width);
    }
    for (const change of [{ pane_width: 199 }, { pane_width: 641 }, { pane_width: 300.5 }, { selected: 7 }, { view: 'unknown' }]) {
      expect(readArrangement({ ...arrangement, operations: { ...operations, ...change } })).toBeNull();
    }
  });
});
