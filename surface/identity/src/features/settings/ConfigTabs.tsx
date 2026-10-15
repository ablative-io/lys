/** Configuration views have their own addresses for the person's account and the service's settings. */
export const CONFIG_TABS: [string, string, string][] = [
  ['account', 'Your account', '#/settings/account'], ['settings', 'This service', '#/settings'], ['connections', 'Services and sign-in providers', '#/connections'], ['apps', 'Apps', '#/apps'],
];

export function ConfigTabs({ on }: { on: string }) {
  return <nav className="tabs config-tabs" aria-label="Configuration views">
    {CONFIG_TABS.map(([key, label, href]) => <a key={key} href={href} className={key === on ? 'on' : ''} aria-current={key === on ? 'page' : undefined}>{label}</a>)}
  </nav>;
}
