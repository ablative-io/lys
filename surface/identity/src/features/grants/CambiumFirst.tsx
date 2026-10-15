/**
 * Said where an agent would be given access and no app's resources are held yet. Which step is missing is read from
 * the apps list: before Cambium is approved, connect it on Apps; once it is approved, the person still needs Cambium
 * access of their own before they can pass any of it on, so they are sent to Access instead of round to Apps again.
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

export function CambiumFirst() {
  const load = useLoad(cambiumApproved, 'cambium-approved');
  // Nothing is said until the apps list answers, so the wrong step is never shown first.
  if (load.status === 'loading') return null;
  if (load.status === 'ok' && load.data) {
    return <p className="note" data-cambium-first="access">Cambium is approved. Give yourself Cambium access on <a href="#/access/issue">Access</a> first; its channels appear here once you hold them.</p>;
  }
  // The apps list is read by the administrator; anyone it refuses is told the first step, which is still true for them.
  return <p className="note" data-cambium-first="apps">Connect Cambium on <a href="#/apps">Apps</a> first; its channels appear here once it is approved.</p>;
}
