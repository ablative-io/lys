import { act } from 'react';
import { describe, expect, it, vi } from 'vitest';
import { CONCEPTS } from '../src/shell/concepts';
import { $, mount, press } from './harness';

/** A ResizeObserver the test drives: it reports a resize only when told to. */
class ResizeObserverDouble {
  static made: ResizeObserverDouble[] = [];
  observed: Element[] = [];
  private readonly callback: ResizeObserverCallback;

  constructor(callback: ResizeObserverCallback) {
    this.callback = callback;
    ResizeObserverDouble.made.push(this);
  }

  observe(el: Element): void {
    this.observed.push(el);
  }

  unobserve(el: Element): void {
    this.observed = this.observed.filter((e) => e !== el);
  }

  disconnect(): void {
    this.observed = [];
  }

  resize(): void {
    act(() => this.callback([], this as unknown as ResizeObserver));
  }
}

describe('explain mode re-measures on a layout signal', () => {
  it('measures once per resize the observer reports, with no timer', async () => {
    let left = 100;
    vi.spyOn(Element.prototype, 'getBoundingClientRect').mockImplementation(
      () => ({ x: left, y: 100, left, top: 100, right: left + 80, bottom: 120, width: 80, height: 20, toJSON: () => ({}) }) as DOMRect,
    );
    ResizeObserverDouble.made = [];
    vi.stubGlobal('ResizeObserver', ResizeObserverDouble);
    await mount('#/people');
    await press('?', {}, document.body);
    expect(ResizeObserverDouble.made).toHaveLength(1);
    const observer = ResizeObserverDouble.made[0];
    expect(observer.observed).toContain(document.documentElement);
    expect(observer.observed.length).toBeGreaterThan(1);
    expect($('.xlayer .xm')?.style.left).toBe('92px');

    const first = CONCEPTS[0].sel;
    const queries = vi.spyOn(document, 'querySelectorAll');
    const measures = () => queries.mock.calls.filter(([sel]) => sel === first).length;
    vi.useFakeTimers();
    try {
      left = 300;
      observer.resize();
      expect(measures()).toBe(1);
      expect($('.xlayer .xm')?.style.left).toBe('292px');
      left = 400;
      observer.resize();
      expect(measures()).toBe(2);
      expect($('.xlayer .xm')?.style.left).toBe('392px');
      expect(vi.getTimerCount()).toBe(0);
    } finally {
      vi.useRealTimers();
    }

    await press('Escape');
    expect($('#xlayer')).toBeNull();
    expect(observer.observed).toEqual([]);
  });
});
