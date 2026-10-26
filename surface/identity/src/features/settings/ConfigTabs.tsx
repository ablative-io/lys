/**
 * Settings views have their own addresses: the person's own settings and service accounts, then the service's,
 * which only administrators may read. Apps is not here: it has its own rail entry.
 */
import { request, useLoad } from '../../api';

export const CONFIG_TABS: [string, string, string][] = [
  ['account', 'For you', '#/settings/account'], ['accounts', 'Service accounts', '#/settings/service-accounts'],
  ['settings', 'This service', '#/settings'], ['connections', 'Services and sign-in providers', '#/connections'],
  ['health', 'Health', '#/health'],
];

/** Everyone's tabs: a person keeps their own service accounts, read by that page alone. */
const EVERYONE = new Set(['account', 'accounts']);

/**
 * The tabs a person may open. "For you" and "Service accounts" are everyone's; the service's tabs are drawn once
 * `/configuration` answers, after them, so nothing the person is reading moves. A tab the person is on is always drawn.
 */
export function ConfigTabs({ on }: { on: string }) {
  const service = useLoad(() => request('/configuration'), 'settings-tabs-configuration');
  const administrator = service.status === 'ok';
  return <nav className="tabs config-tabs" aria-label="Settings views">
    {CONFIG_TABS.filter(([key]) => EVERYONE.has(key) || key === on || administrator).map(([key, label, href]) => <a key={key} href={href} className={key === on ? 'on' : ''} aria-current={key === on ? 'page' : undefined}>{label}</a>)}
  </nav>;
}
