/** Configuration is one page. Its views are tabs, each with its own address: how this service runs, the services and sign-in providers it uses, and the apps that sign in with it. */
export const CONFIG_TABS: [string, string, string][] = [
  ['settings', 'This service', '#/settings'], ['connections', 'Services and sign-in providers', '#/connections'], ['apps', 'Apps', '#/apps'],
];

export function ConfigTabs({ on }: { on: string }) {
  return <nav className="tabs" aria-label="Configuration views">
    {CONFIG_TABS.map(([key, label, href]) => <a key={key} href={href} className={key === on ? 'on' : ''} aria-current={key === on ? 'page' : undefined}>{label}</a>)}
  </nav>;
}
