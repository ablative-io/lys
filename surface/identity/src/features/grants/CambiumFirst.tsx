/** Said where an agent would be given access and no app's resources exist yet: on a fresh install, Cambium's channels
 * only appear once Cambium is connected and approved on Apps. */
import type { ResourceRef } from '../../generated/grants';

/** Whether any of `resources` belongs to an app (an app kind is `app.kind`). */
export const anyAppResource = (resources: ResourceRef[]): boolean => resources.some((resource) => resource.kind.includes('.'));

export function CambiumFirst() {
  return <p className="note" data-cambium-first>Connect Cambium on <a href="#/apps">Apps</a> first; its channels appear here once it is approved.</p>;
}
