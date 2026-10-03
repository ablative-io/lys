import { describe, expect, it, vi } from 'vitest';
import { CONCEPTS } from '../src/shell/concepts';
import { $, $$, click, mount, press, unreachable } from './harness';

// jsdom lays nothing out; give every element a box on screen so explain mode can place its numbers.
function layOut() {
  vi.spyOn(Element.prototype, 'getBoundingClientRect').mockReturnValue({
    x: 100, y: 100, left: 100, top: 100, right: 180, bottom: 120, width: 80, height: 20, toJSON: () => ({}),
  } as DOMRect);
}

describe('help overlay (conformance 9.2)', () => {
  it('numbers what is on screen', async () => {
    layOut();
    await mount('#/people');
    await press('?', {}, document.body);
    const present = CONCEPTS.filter((c) => $$(c.sel).some((e) => !e.closest('.xlayer')));
    const marks = $$('.xlayer .xm');
    expect(marks.length).toBeGreaterThanOrEqual(3);
    expect(marks.length).toBe(present.length);
    expect(marks.map((m) => m.textContent)).toEqual(marks.map((_, i) => String(i + 1)));
    expect($('.xbar')?.textContent).toContain(`${marks.length} things explained`);
    expect(document.activeElement).toBe(marks[0]);
  });

  it('dismissing never presses what is underneath', async () => {
    layOut();
    await mount('#/people');
    const pressed = vi.fn();
    document.addEventListener('click', pressed);
    await press('?', {}, document.body);
    await click($('[data-xabsorb]'));
    expect($('#xlayer')).toBeNull();
    expect(pressed).not.toHaveBeenCalled();
    expect($('.toast.show')).toBeNull();
    await press('?', {}, document.body);
    await click($('[data-xoff]'));
    expect($('#xlayer')).toBeNull();
    expect(pressed).not.toHaveBeenCalled();
    document.removeEventListener('click', pressed);
  });

  it('Escape exits and focus returns to where it was', async () => {
    layOut();
    await mount('#/people');
    const row = $('tr[data-pick="1"]');
    row?.focus();
    await press('?');
    expect($('#xlayer')).not.toBeNull();
    await press('Escape');
    expect($('#xlayer')).toBeNull();
    expect(document.activeElement).toBe(row);
    expect(location.hash).toBe('#/people');
  });

  it('a number opens its concept in the help dock', async () => {
    layOut();
    await mount('#/people');
    await press('?', {}, document.body);
    const mark = $$('.xlayer .xm').find((m) => m.dataset.xm === 'state');
    await click(mark ?? null);
    expect($('#xlayer')).toBeNull();
    expect($('#dock')?.classList.contains('open')).toBe(true);
    expect($('.dock-head b')?.textContent).toBe('State');
    expect($('.sheet')?.textContent).toContain('Authority only');
  });

  it('the help dock is searchable and usable by keyboard', async () => {
    await mount('#/people');
    await click($('[data-dockbtn="help"]'));
    expect($$('#helpList .concept').length).toBe(CONCEPTS.length);
    expect(unreachable()).toEqual([]);
    const concept = $('#helpList .concept[data-help="rail"]');
    concept?.focus();
    await press('Enter');
    expect($('.dock-head b')?.textContent).toBe('The rail');
    await click($('[data-help=""]'));
    expect($('#helpQ')).not.toBeNull();
    await click($('[data-dockbtn-close]'));
    expect($('#dock')?.classList.contains('open')).toBe(false);
  });

  it('offers nothing that is not built: there is no assistant button', async () => {
    await mount('#/people');
    expect($('[data-dockbtn="assistant"]')).toBeNull();
    expect(document.body.textContent).not.toContain('not built yet');
  });
});
