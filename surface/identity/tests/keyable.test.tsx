import userEvent from '@testing-library/user-event';
import { act } from 'react';
import { createRoot } from 'react-dom/client';
import { describe, expect, it } from 'vitest';
import { keyable } from '../src/shell/keyable';
import { ADA, BEA, SCRIBE } from './fixtures';
import { $, $$, click, mount, press, settle } from './harness';

// Conformance 9.3: whatever acts on a click answers Enter and Space by one click
// on itself (index.v5.html:1559-1562), palette rows included (index.v5.html:1103).

/** Count the native clicks that reach `el`. */
function clicks(el: Element): { count: number } {
  const seen = { count: 0 };
  el.addEventListener('click', () => {
    seen.count += 1;
  });
  return seen;
}

async function openPalette(): Promise<HTMLInputElement> {
  await press('k', { metaKey: true }, document.body);
  expect($('#palette')?.classList.contains('open')).toBe(true);
  const input = $('#palIn');
  if (!(input instanceof HTMLInputElement)) throw new Error('no palette input');
  expect(document.activeElement).toBe(input);
  return input;
}

async function tab(): Promise<void> {
  const user = userEvent.setup();
  await act(async () => {
    await user.tab();
  });
  await settle();
}

const paletteRows = () => $$('#palette .it[data-n]');

describe('keyable (conformance 9.3)', () => {
  it('keyable answers Enter Space and click once each', async () => {
    let calls = 0;
    const action = () => {
      calls += 1;
    };
    const container = document.createElement('div');
    document.body.appendChild(container);
    const root = createRoot(container);
    try {
      await act(async () => {
        root.render(<div id="keyed" {...keyable(action)}>act</div>);
      });
      const div = $('#keyed');
      if (!div) throw new Error('keyable div not rendered');
      expect(div.tabIndex).toBe(0);
      expect(div.getAttribute('role')).toBe('button');
      const seen = clicks(div);
      div.focus();
      expect(document.activeElement).toBe(div);
      await press('Enter', {}, div);
      expect([seen.count, calls]).toEqual([1, 1]);
      await press(' ', {}, div);
      expect([seen.count, calls]).toEqual([2, 2]);
      await click(div);
      expect([seen.count, calls]).toEqual([3, 3]);
    } finally {
      act(() => root.unmount());
    }
  });

  it('Enter and Space on a People row open its file', async () => {
    await mount('#/people');
    for (const [key, id] of [['Enter', BEA], [' ', SCRIBE]]) {
      const row = $(`tr[data-href="#/file/${id}"]`);
      if (!row) throw new Error('no People row for ' + id);
      const seen = clicks(row);
      row.focus();
      expect(document.activeElement).toBe(row);
      await press(key, {}, row);
      expect(seen.count, JSON.stringify(key)).toBe(1);
      expect(location.hash).toBe('#/file/' + id);
      await act(async () => {
        location.hash = '#/people';
      });
      await settle();
    }
  });

  it('Enter on a focused palette row chooses that row', async () => {
    await mount('#/access');
    const input = await openPalette();
    expect(paletteRows().length).toBeGreaterThanOrEqual(2);
    await press('ArrowDown', {}, input);
    expect(paletteRows()[1].classList.contains('sel')).toBe(true);
    expect(paletteRows()[1].textContent).toContain('Bea (test person)');
    await tab();
    const first = paletteRows()[0];
    expect(document.activeElement).toBe(first);
    expect(first.textContent).toContain('Ada (test person)');
    await press('Enter', {}, first);
    expect(location.hash).toBe('#/file/' + ADA);
    expect($('#palette')?.classList.contains('open')).toBe(false);
  });

  it('Space on a focused palette row chooses it', async () => {
    await mount('#/access');
    await openPalette();
    await tab();
    await tab();
    const second = paletteRows()[1];
    expect(document.activeElement).toBe(second);
    expect(second.classList.contains('sel')).toBe(false);
    await press(' ', {}, second);
    expect(location.hash).toBe('#/file/' + BEA);
    expect($('#palette')?.classList.contains('open')).toBe(false);
  });

  it('arrows and Enter in the palette input still choose', async () => {
    await mount('#/access');
    const input = await openPalette();
    await press('ArrowDown', {}, input);
    await press('ArrowDown', {}, input);
    expect(paletteRows()[2].classList.contains('sel')).toBe(true);
    expect(paletteRows()[2].textContent).toContain('Scribe');
    await press('Enter', {}, input);
    expect(location.hash).toBe('#/file/' + SCRIBE);
    expect($('#palette')?.classList.contains('open')).toBe(false);
  });

  it('closed palette rows stay out of the Tab order', async () => {
    await mount('#/people');
    expect($('#palette')?.getAttribute('aria-hidden')).toBe('true');
    expect($('#palIn')?.tabIndex).toBe(-1);
    const rows = paletteRows();
    expect(rows.length).toBeGreaterThan(0);
    expect(rows.filter((row) => row.tabIndex === 0)).toEqual([]);
  });
});
