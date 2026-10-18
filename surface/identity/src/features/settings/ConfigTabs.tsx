/** Settings views have their own addresses: the person's own settings, then the service's, which only administrators may read. */
import { request, useLoad } from '../../api';

export const CONFIG_TABS: [string, string, string][] = [
  ['account', 'For you', '#/settings/account'], ['settings', 'This service', '#/settings'], ['connections', 'Services and sign-in providers', '#/connections'], ['apps', 'Apps', '#/apps'],
];

/**
 * The tabs a person may open. "For you" is everyone's; the service's tabs are drawn once `/configuration` answers,
 * after "For you", so nothing the person is reading moves. A tab the person is on is always drawn.
 */
export function ConfigTabs({ on }: { on: string }) {
  const service = useLoad(() => request('/configuration'), 'settings-tabs-configuration');
  const administrator = service.status === 'ok';
  return <nav className="tabs config-tabs" aria-label="Settings views">
    {CONFIG_TABS.filter(([key]) => key === 'account' || key === on || administrator).map(([key, label, href]) => <a key={key} href={href} className={key === on ? 'on' : ''} aria-current={key === on ? 'page' : undefined}>{label}</a>)}
  </nav>;
}
