import { useEffect, useRef, useState } from 'react';
import { api } from '../api';
import { subscribeChanges } from '../live';
import { Act } from './Act';

/** How the page is reloaded; a test stands in for the browser's own. */
export const page = { reload: (): void => location.reload() };

/**
 * A tab left open across an upgrade goes on running the screens it loaded. The bundle does not know
 * the commit it was built from, so the first build the service answers at /authority is this page's
 * own. Whenever the change feed answers (established, re-established after it broke, as it does
 * when the service restarts, or with a new generation) the build is read again; once it differs,
 * the page says so and offers a reload. No clock: the feed is the only signal.
 */
export function Upgraded() {
  const first = useRef<string | null>(null);
  const [upgraded, setUpgraded] = useState(false);
  useEffect(() => {
    let live = true;
    const read = () => {
      api.authority().then(
        ({ build }) => {
          if (!live) return;
          first.current ??= build;
          if (build !== first.current) setUpgraded(true);
        },
        // Whether the service was upgraded is not known from a refused read; the pages say the
        // refusal themselves, and the next answer of the feed asks again.
        () => undefined,
      );
    };
    const unsubscribe = subscribeChanges(read, () => undefined);
    return () => { live = false; unsubscribe(); };
  }, []);
  if (!upgraded) return null;
  return (
    <div className="upgraded" role="status">
      <span>Lys was upgraded. Reload to use the new screens.</span>
      <Act symbol="retry" name="Reload" word="Reload" tone="primary" onClick={() => page.reload()} />
    </div>
  );
}
