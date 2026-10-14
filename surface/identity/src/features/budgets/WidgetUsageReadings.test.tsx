import { act } from 'react';
import { createRoot } from 'react-dom/client';
import { afterEach, describe, expect, it, vi } from 'vitest';
import type { DashboardAgent } from '../dashboard/contract';
import type { Used } from '../usage/contract';
import { ContextReadings, TokenReadings, WidgetUsageReadings, usageSummary } from './WidgetUsageReadings';
import type { ContextUsed, SessionContext } from './widgetReadings';
import type { TokenComponents, TokensUsed } from './tokenReadings';

const at = Math.floor(1_792_000_000_000 / 18_000_000) * 18_000_000;
const model = 'claude-opus-5-5';
const older: SessionContext = { session: 'older-live', read_at_ms: at - 600_000, reported_none: 8, model,
  context_tokens: 980_000, window_tokens: 1_000_000, figure: 98, unavailable: null };
const current: SessionContext = { session: 'current-live', read_at_ms: at, reported_none: 2, model,
  context_tokens: 58_795, window_tokens: 1_000_000, figure: 5, unavailable: null };
const context = (live = [older, current]): ContextUsed => ({ unit: 'context_percent', period: null, since_ms: null,
  figure: 5, unavailable: null, source: 'model', context: { ...current, live_sessions: live } });
const components: TokenComponents = { input_tokens: { reported: 500, missing_calls: 0 }, output_tokens: { reported: 20, missing_calls: 0 },
  cache_creation_tokens: { reported: 100, missing_calls: 0 }, cache_read_tokens: { reported: 58_195, missing_calls: 1 },
  calls: 2, sessions: 1, calls_without_session: 0 };
const tokens = (period: 'day' | 'five_hour' = 'day'): TokensUsed => ({ unit: 'tokens', period, since_ms: at,
  figure: 177_540, unavailable: null, tokens: components });
const row = (id: string, figures: Used[], accounts: unknown[] = []): DashboardAgent => ({
  agent: { id, display_name: id }, teams: [], sessions: [], goals: { goals: [] },
  usage: { agent: id, used: [], receipts: [], last_reported_ms: at, figures, accounts },
} as DashboardAgent);
let root: ReturnType<typeof createRoot> | undefined;
let host: HTMLElement;
afterEach(() => { if (root) act(() => root!.unmount()); root = undefined; host?.remove(); });
async function render(element: React.ReactNode) {
  host = document.createElement('div'); document.body.append(host); root = createRoot(host);
  await act(async () => { root!.render(element); });
  return host;
}

describe('Context session choices on the canvas usage widget', () => {
  it('names the current live session and shows its 5.9 percent, read time, raw tokens and declared window', async () => {
    const host = await render(<ContextReadings rows={[row('Agent', [context()])]} />);
    expect(host.querySelector('[data-context-session]')?.getAttribute('data-context-session')).toBe('current-live');
    expect(host.querySelector('[data-context-percent]')?.textContent).toBe('5.9%');
    expect(host.querySelector('time')?.getAttribute('datetime')).toBe(new Date(at).toISOString());
    expect(host.querySelector('[data-context-tokens]')?.textContent).toBe('58,795 tokens reported of the declared 1,000,000 tokens window');
    expect(host.querySelector('select')?.getAttribute('aria-label')).toBe('Context session for Agent');
    expect(host.textContent).toContain('2 calls reported none');
  });

  it('allows the person to pick another named live session and shows that session reading instead of a maximum', async () => {
    const host = await render(<ContextReadings rows={[row('Agent', [context()])]} />);
    const pick = host.querySelector('select')!;
    await act(async () => { pick.value = older.session; pick.dispatchEvent(new Event('change', { bubbles: true })); });
    expect(host.querySelector('[data-context-session]')?.getAttribute('data-context-session')).toBe(older.session);
    expect(host.querySelector('[data-context-percent]')?.textContent).toBe('98%');
    expect(host.querySelector('time')?.getAttribute('datetime')).toBe(new Date(older.read_at_ms!).toISOString());
    expect(host.textContent).toContain('8 calls reported none');
  });

  it('names an ended picked session on reload of usage instead of switching the person selection silently', async () => {
    const host = await render(<ContextReadings rows={[row('Agent', [context()])]} />);
    const pick = host.querySelector('select')!;
    await act(async () => { pick.value = older.session; pick.dispatchEvent(new Event('change', { bubbles: true })); });
    await act(async () => { root!.render(<ContextReadings rows={[row('Agent', [context([current])])]} />); });
    expect(host.textContent).toContain('the selected session is no longer live');
    expect(host.querySelector('[data-context-percent]')).toBeNull();
    expect(host.querySelector('select')?.value).toBe(older.session);
  });

  it('supports a controlled canvas choice without overriding the widget owner selection', async () => {
    const select = vi.fn();
    const host = await render(<ContextReadings rows={[row('Agent', [context()])]} selected={{ Agent: older.session }} select={select} />);
    const pick = host.querySelector('select')!;
    await act(async () => { pick.value = current.session; pick.dispatchEvent(new Event('change', { bubbles: true })); });
    expect(select).toHaveBeenCalledWith('Agent', current.session);
    expect(host.querySelector('[data-context-session]')?.getAttribute('data-context-session')).toBe(older.session);
  });
});

describe('Token classes and provider windows on the canvas usage widget', () => {
  it('renders four token classes and call/session counts per agent without displaying the unrelated combined scalar', async () => {
    const host = await render(<TokenReadings rows={[row('One', [tokens()]), row('Other', [tokens()])]} />);
    expect(host.querySelectorAll('[data-token-agent]')).toHaveLength(2);
    for (const section of host.querySelectorAll('[data-token-agent]')) {
      expect(Array.from(section.querySelectorAll('th')).map((each) => each.textContent)).toEqual(['New input', 'Output', 'Cache written', 'Cache read']);
      expect(section.querySelector('[data-token-counts]')?.textContent).toBe('2 calls · 1 session');
      expect(section.querySelector('[data-token-component="cache_read_tokens"]')?.textContent).toContain('58,195 tokens reported · 1 call reported none');
    }
    expect(host.textContent).not.toContain('177,540');
  });

  it('names a missing provider window and shows recorded interval components without inventing a plan percentage', async () => {
    const host = await render(<WidgetUsageReadings rows={[row('Agent', [tokens('five_hour')])]} keys={['window/300']} />);
    expect(host.querySelector('[data-provider-window-unavailable]')?.textContent).toBe('5-hour provider window not reported');
    expect(host.textContent).toContain('Tokens in this 5-hour interval');
    expect(host.querySelectorAll('[data-token-component]')).toHaveLength(4);
    expect(host.textContent).not.toContain('%');
    expect(host.textContent).not.toContain('177,540');
  });

  it('shows an actual reported provider window and reset separately from token components', async () => {
    const account = { account: 'org', at_ms: at, windows: [{ duration_minutes: 300, used_percent: 30, resets_at_ms: at + 18_000_000 }] };
    const host = await render(<WidgetUsageReadings rows={[row('Agent', [tokens('five_hour')], [account])]} keys={['window/300']} />);
    expect(host.querySelector('[data-provider-window]')?.textContent).toContain('30%');
    expect(host.querySelector('[data-provider-window-unavailable]')).toBeNull();
  });

  it('names an invalid provider level instead of inventing or clamping its window percentage', async () => {
    const account = { account: 'org', at_ms: at, windows: [{ duration_minutes: 300, used_percent: 130, resets_at_ms: at + 18_000_000 }] };
    const host = await render(<WidgetUsageReadings rows={[row('Agent', [], [account])]} keys={['window/300']} />);
    expect(host.querySelector('[role="alert"]')?.textContent).toBe('ProviderWindowUnreadable: the reported account, level or time is invalid');
    expect(host.querySelector('[data-provider-window]')).toBeNull();
  });

  it('keeps a known one-percent provider account beside the exact reason another account did not report its window', async () => {
    const reason = '300-minute plan window unreported for account unknown';
    const gap: Used = { unit: 'plan_percent', period: 'five_hour', since_ms: null, figure: null, unavailable: reason };
    const account = { account: 'known', at_ms: at, windows: [{ duration_minutes: 300, used_percent: 1, resets_at_ms: at + 18_000_000 }] };
    const host = await render(<WidgetUsageReadings rows={[row('Agent', [gap], [account])]} keys={['window/300']} />);
    expect(host.querySelector('[data-provider-window]')?.textContent).toContain('1%');
    expect(host.querySelector('[data-provider-window-gaps]')?.textContent).toBe('Agent: ' + reason);
    expect(usageSummary([row('Agent', [gap], [account])], 'window/300')?.value).toBeUndefined();
  });

  it('keeps the exact reset reason when no account window remains and shows period components separately', async () => {
    const reason = 'the plan window reset at ' + at + '; no report since';
    const gap: Used = { unit: 'plan_percent', period: 'five_hour', since_ms: at, figure: null, unavailable: reason };
    const host = await render(<WidgetUsageReadings rows={[row('Agent', [gap, tokens('five_hour')])]} keys={['window/300']} />);
    expect(host.querySelector('[data-provider-window-gaps]')?.textContent).toBe('Agent: ' + reason);
    expect(host.querySelector('[data-provider-window-unavailable]')?.textContent).toBe('5-hour provider window not reported');
    expect(host.querySelectorAll('[data-token-component]')).toHaveLength(4);
    expect(host.textContent).not.toContain('177,540');
  });

  it('supplies scalar formula input for the selected session because context is never a cross-session maximum or component token total', () => {
    expect(usageSummary([row('Agent', [context()])], 'context_percent/')).toMatchObject({ percent: 5.8795,
      value: { amount: 5.8795, unit: 'context_percent' } });
    expect(usageSummary([row('Agent', [context()])], 'context_percent/', { Agent: older.session })).toMatchObject({ percent: 98,
      value: { amount: 98, unit: 'context_percent' } });
    expect(usageSummary([row('Agent', [context([current])])], 'context_percent/', { Agent: older.session })?.value).toBeUndefined();
    expect(usageSummary([row('One', [context()]), row('Other', [context()])], 'context_percent/')?.value).toBeUndefined();
    expect(usageSummary([row('Agent', [tokens()])], 'tokens/day')?.value).toBeUndefined();
    expect(usageSummary([row('Agent', [tokens()])], 'tokens/day')?.figure).not.toContain('177,540');
  });
});
