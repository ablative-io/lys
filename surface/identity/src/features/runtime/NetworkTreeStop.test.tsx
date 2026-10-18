import { act, useState } from 'react';
import { createRoot } from 'react-dom/client';
import { expect, it, vi } from 'vitest';
import { NetworkTree } from './NetworkTree';
import { ok, SCRIBE, SCRIBE_VIEW, SERVICE } from '../../../tests/fixtures';
import { serve, settle } from '../../../tests/harness';

const live = vi.hoisted(() => { vi.resetModules(); return { sessions: [{ session: 'live-session', agent: 'agent', machine: 'computer', shown: 'running' }] }; });
vi.mock('../../api', async (original) => ({ ...await original<typeof import('../../api')>(),
  useLive: (_read: unknown, key: string) => ({ status: 'ok', data: key === 'operations-network-running'
    ? { sessions: live.sessions, unanswered: null }
    : { runner: 'read', seats: [], machines: [{ id: 'computer', name: 'Computer', kind: 'desktop', state: 'in_use', runtime: 'lys-runner', may_run: [{ id: SCRIBE, display_name: SCRIBE_VIEW.display_name, state: 'active' }] }] } }),
}));
vi.mock('./Terminal', () => ({ Terminal: () => <p>Running terminal</p> }));

it('returns to the selected agent after its live session ends so Start stays in the same pane', async () => {
  live.sessions = [{ session: 'live-session', agent: SCRIBE, machine: 'computer', shown: 'running' }];
  serve({ ...SERVICE, ['/agents/' + SCRIBE + '/provisioning']: ok({ agent: SCRIBE, profile: null, versions: [], enforced: false }) });
  const host = document.createElement('div');
  document.body.append(host);
  const root = createRoot(host);
  const selected = vi.fn();
  function Tree() {
    const [choice, choose] = useState('session:live-session');
    return <NetworkTree selected={choice} select={(value) => { selected(value); choose(value); }} paneWidth={null} resize={() => undefined} />;
  }
  try {
    await act(async () => { root.render(<Tree />); });
    expect(host.textContent).toContain('Running terminal');
    live.sessions = [];
    await act(async () => { root.render(<Tree />); });
    await settle();
    expect(selected).toHaveBeenCalledWith('agent:' + SCRIBE);
    expect(host.querySelector('.network-pane .run-start')).not.toBeNull();
    expect(host.textContent).not.toContain('The selected session is not running');
  } finally { act(() => root.unmount()); host.remove(); }
});
