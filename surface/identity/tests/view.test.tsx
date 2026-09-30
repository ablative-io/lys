/** The watched terminals follow the person from page to page as a small picture, open into a multiplexer, hide without losing the set, and never resize a session while small. */
import { describe, expect, it, vi } from 'vitest';
import { act } from 'react';
import { $, $$, click, mount, settle, text } from './harness';
import { ADA, SCRIBE, SERVICE, ok } from './fixtures';
import { mockTerminal } from './terminal-double';
vi.mock('@gespenst/core', () => ({ createTerminal: mockTerminal }));

const LIVE = 'op-' + '5'.repeat(32);
const LAB = 'op-' + 'b'.repeat(32);
const running = { session: LIVE, agent: SCRIBE, machine: LAB, machine_name: 'Lab', runtime: 'lys-runner', shown: 'running', last_reported: 'running', first_report_at: 1790000000, last_report_at: 1790000001, what: 'started', stopped: null, reported_by: 'the runner' };
const routes = { ...SERVICE, '/runtime/live': ok({ sessions: [running], unanswered: [] }), ['POST /runtime/sessions/' + LIVE + '/resize']: ok({ session: LIVE, answer: { kind: 'resized' }, receipt: { index: 1 } }) };

describe('Watched view', () => {
  it('watches an agent from the front page, keeps it across pages, opens and hides it', async () => {
    localStorage.clear();
    const { posted } = await mount('#/me', routes);
    expect($('.view')).toBeNull();
    await click($$('.you-watch').find((el) => el.textContent === 'Watch') ?? null);
    expect($('.view-small')).not.toBeNull();
    expect($$('.view-tab [role="tab"]').map((el) => el.textContent)).toEqual(['Scribe']);
    expect(JSON.parse(localStorage.getItem('iam.view') ?? '[]')).toMatchObject([{ session: LIVE, agent: SCRIBE, name: 'Scribe', machine: 'Lab' }]);
    expect(posted.filter((call) => call.path.endsWith('/resize'))).toEqual([]);

    location.hash = '#/roles';
    await new Promise((resolve) => setTimeout(resolve, 0));
    expect($('.view-small')).not.toBeNull();

    await click($$('.view-act').find((el) => el.textContent === 'Open') ?? null);
    expect($('.view-open')).not.toBeNull();
    expect($('.view-pane-name')?.textContent).toBe('Scribeon Lab'.replace('Scribeon', 'Scribe on'));
    expect(localStorage.getItem('iam.view-state')).toBe('open');
    await click($$('.view-act').find((el) => el.textContent === 'Back to the page') ?? null);
    expect($('.view-small')).not.toBeNull();

    await click($$('.view-act').find((el) => el.textContent === 'Hide') ?? null);
    expect($('.view-hidden')?.textContent).toBe('1 agent watched');
    expect(JSON.parse(localStorage.getItem('iam.view') ?? '[]')).toHaveLength(1);
    await click($('.view-hidden'));
    expect($('.view-small')).not.toBeNull();

    await click($('.view-x'));
    expect($('.view')).toBeNull();
    expect(localStorage.getItem('iam.view')).toBe('[]');
    expect(text()).not.toContain('undefined');
    localStorage.clear();
  });

  it('resizes by its corner and becomes a real terminal at full size', async () => {
    localStorage.clear();
    localStorage.setItem('iam.view', JSON.stringify([{ session: LIVE, agent: SCRIBE, name: 'Scribe', machine: 'Lab' }]));
    await mount('#/roles', routes);
    const grip = $('.view-grip') as HTMLButtonElement;
    expect(grip).not.toBeNull();
    grip.setPointerCapture = () => undefined;
    grip.releasePointerCapture = () => undefined;
    await act(async () => {
      grip.dispatchEvent(new PointerEvent('pointerdown', { clientX: 1000, bubbles: true, pointerId: 1 }));
      grip.dispatchEvent(new PointerEvent('pointermove', { clientX: 600, bubbles: true, pointerId: 1 }));
      grip.dispatchEvent(new PointerEvent('pointerup', { clientX: 600, bubbles: true, pointerId: 1 }));
    });
    await settle();
    expect(localStorage.getItem('iam.view-width')).toBe('740');
    expect($('.view-real')).not.toBeNull();
    expect($('.view-picture')).toBeNull();
    localStorage.clear();
  });

  it('ignores a stored view it cannot read', async () => {
    localStorage.setItem('iam.view', '{not json');
    await mount('#/roles', routes);
    expect($('.view')).toBeNull();
    localStorage.clear();
  });
});

void ADA;
