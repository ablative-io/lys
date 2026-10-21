/**
 * Said where an agent would be given access and no app's resources are held yet. Which step is missing is read from
 * the apps list: before any app is approved, connect one on Apps; once one is approved, the person still needs access
 * of their own before they can pass any of it on, so they are sent to Access instead of round to Apps again. No app is
 * named here: which apps there are is the apps list's to say.
 * Only the administrator (the root authority, the one the whole directory is answered to) connects an app or gives
 * access outright, so anyone else is told who does it, never sent to a page they cannot use.
 */
import { request, useLoad } from '../../api';
import type { ResourceRef } from '../../generated/grants';

/** Whether any of `resources` belongs to an app (an app kind is `app.kind`). */
export const anyAppResource = (resources: ResourceRef[]): boolean => resources.some((resource) => resource.kind.includes('.'));

/** The ids of the apps the apps list answers as approved, in its order. Any other answer, or none, is read as none. */
export async function approvedApps(): Promise<string[]> {
  const { apps } = await request<{ apps: { id: string; state: string }[] }>('/apps');
  return apps.filter((app) => app.state === 'approved').map((app) => app.id);
}

/** Whether the signed-in person is the administrator: the whole directory is answered to them alone. */
const administrator = (): Promise<boolean> => request('/directory/people').then(() => true, () => false);

export function AppsFirst() {
  const load = useLoad(approvedApps, 'apps-approved');
  const root = useLoad(administrator, 'apps-first-administrator');
  // Nothing is said until both answer, so the wrong step is never shown first.
  if (load.status === 'loading' || root.status !== 'ok') return null;
  const approved = load.status === 'ok' ? load.data : [];
  if (approved.length) {
    const issue = approved.length === 1 ? '#/access/issue?app=' + encodeURIComponent(approved[0] ?? '') : '#/access/issue';
    return root.data
      ? <p className="note" data-apps-first="access">An app is approved. Give yourself access to it on <a href={issue}>Access</a> first; what it shares appears here once you hold it.</p>
      : <p className="note" data-apps-first="access">Ask an administrator for access to an app; what it shares appears here once you hold it.</p>;
  }
  return root.data
    ? <p className="note" data-apps-first="apps">Connect an app on <a href="#/apps">Apps</a> first; what it shares appears here once it is approved.</p>
    : <p className="note" data-apps-first="apps">No app is connected to Lys yet. An administrator connects one on Apps.</p>;
}
