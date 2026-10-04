/** Access is one page. Its views are tabs, each with its own address: the grants, the three questions, requests, drafts an agent prepared for its person, reviews, resources and the model. The graph has its own place on the rail. Tabs sit under the page's title. */
export const ACCESS_TABS: [string, string, string][] = [
  ['grants', 'Grants', '#/access'], ['ask', 'Ask', '#/access/can'], ['requests', 'Requests', '#/requests'], ['drafts', 'Drafts', '#/access/drafts'], ['reviews', 'Reviews', '#/reviews'],
  ['resources', 'Resources', '#/resources'], ['model', 'Model', '#/model'],
];

export function AccessTabs({ on }: { on: string }) {
  return <nav className="tabs" aria-label="Access views">
    {ACCESS_TABS.map(([key, label, href]) => <a key={key} href={href} className={key === on ? 'on' : ''} aria-current={key === on ? 'page' : undefined}>{label}</a>)}
  </nav>;
}
