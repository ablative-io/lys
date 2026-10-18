/** Access is one page. Its views are tabs, each with its own address: the grants, the three questions, requests, drafts an agent prepared for its person, reviews, resources, the model, the access map and jobs. Neither of the last two has a place on the rail, so these tabs are their way in. Tabs sit under the page's title. */
export const ACCESS_TABS: [string, string, string][] = [
  ['grants', 'Grants', '#/access'], ['ask', 'Ask', '#/access/can'], ['requests', 'Requests', '#/requests'], ['drafts', 'Drafts', '#/access/drafts'], ['reviews', 'Reviews', '#/reviews'],
  ['resources', 'Resources', '#/resources'], ['model', 'Model', '#/model'], ['graph', 'Access map', '#/graph'], ['roles', 'Jobs', '#/roles'],
];

export function AccessTabs({ on }: { on: string }) {
  return <nav className="tabs" aria-label="Access views">
    {ACCESS_TABS.map(([key, label, href]) => <a key={key} href={href} className={key === on ? 'on' : ''} aria-current={key === on ? 'page' : undefined}>{label}</a>)}
  </nav>;
}
