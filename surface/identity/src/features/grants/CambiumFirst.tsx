/**
 * Said where an agent would be given access and no app's resources are held yet. Which step is missing is read from
 * the apps list: before Cambium is approved, connect it on Apps; once it is approved, the person still needs Cambium
 * access of their own before they can pass any of it on, so they are sent to Access instead of round to Apps again.
 * Only the administrator (the root authority, the one the whole directory is answered to) connects an app or gives
 * access outright, so anyone else is told who does it, never sent to a page they cannot use.
 */
import { request, useLoad } from '../../api';
import type { ResourceRef } from '../../generated/grants';

/** Whether any of `resources` belongs to an app (an app kind is `app.kind`). */
export const anyAppResource = (resources: ResourceRef[]): boolean => resources.some((resource) => resource.kind.includes('.'));

const CAMBIUM = 'cambium';

/** Whether the apps list answers Cambium as approved. Any other answer, or none, is read as not approved yet. */
export async function cambiumApproved(): Promise<boolean> {
  const { apps } = await request<{ apps: { id: string; state: string }[] }>('/apps');
  return apps.some((app) => app.id === CAMBIUM && app.state === 'approved');
}

/** Whether the signed-in person is the administrator: the whole directory is answered to them alone. */
const administrator = (): Promise<boolean> => request('/directory/people').then(() => true, () => false);

export function CambiumFirst() {
  const load = useLoad(cambiumApproved, 'cambium-approved');
  const root = useLoad(administrator, 'cambium-first-administrator');
  // Nothing is said until both answer, so the wrong step is never shown first.
  if (load.status === 'loading' || root.status !== 'ok') return null;
  const approved = load.status === 'ok' && load.data;
  if (approved) {
    return root.data
      ? <p className="note" data-cambium-first="access">Cambium is approved. Give yourself Cambium access on <a href="#/access/issue?app=cambium">Access</a> first; its channels appear here once you hold them.</p>
      : <p className="note" data-cambium-first="access">Cambium is approved. Ask an administrator for Cambium access; its channels appear here once you hold it.</p>;
  }
  return root.data
    ? <p className="note" data-cambium-first="apps">Connect Cambium on <a href="#/apps">Apps</a> first; its channels appear here once it is approved.</p>
    : <p className="note" data-cambium-first="apps">Cambium is not connected to Lys yet. An administrator connects it on Apps; its channels appear here once it is.</p>;
}
