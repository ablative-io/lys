import { describe, expect, it } from 'vitest';
import { $, $$, mount, press } from './harness';

// The keyboard cursor moves focus with it and keeps the screen's elements
// (index.v5.html:1132-1133); Enter opens the focused row, or the cursor row
// when no row has focus.

const rows = () => $$('#screen tr[data-href]');

async function people(): Promise<HTMLElement[]> {
  await mount('#/people');
  const list = rows();
  expect(list.length).toBeGreaterThanOrEqual(3);
  expect(list[0].classList.contains('cursor')).toBe(true);
  if (document.activeElement instanceof HTMLElement) document.activeElement.blur();
  expect(document.activeElement).toBe(document.body);
  return list;
}

describe('keyboard cursor (conformance 9.3)', () => {
  it('j moves focus with the cursor without re-rendering', async () => {
    const list = await people();
    const first = $('#screen')?.firstElementChild;
    expect(first).toBeTruthy();
    await press('j');
    expect(document.activeElement).toBe(list[1]);
    await press('j');
    expect(document.activeElement).toBe(list[2]);
    expect(list[2].classList.contains('cursor')).toBe(true);
    expect(rows().filter((row) => row.classList.contains('cursor'))).toEqual([list[2]]);
    expect($('#screen')?.firstElementChild).toBe(first);
    expect(rows()).toEqual(list);
  });

  it('k moves focus back one row', async () => {
    const list = await people();
    await press('j');
    await press('j');
    await press('k');
    expect(document.activeElement).toBe(list[1]);
    expect(list[1].classList.contains('cursor')).toBe(true);
    expect(rows().filter((row) => row.classList.contains('cursor'))).toEqual([list[1]]);
  });

  it('Enter opens the focused row', async () => {
    const list = await people();
    const firstHref = list[0].dataset.href;
    const secondHref = list[1].dataset.href;
    expect(secondHref).not.toBe(firstHref);
    await press('j');
    expect(document.activeElement).toBe(list[1]);
    await press('Enter');
    expect(location.hash).toBe(secondHref);
    expect(location.hash).not.toBe(firstHref);
  });

  it('Enter with no row focused opens the cursor row', async () => {
    const list = await people();
    await press('Enter', {}, document.body);
    expect(location.hash).toBe(list[0].dataset.href);
    expect(location.hash).toMatch(/^#\/file\//);
  });
});
