/**
 * The connections the install records itself, with no grant behind them: the
 * administrator on the directory, and each registered app signing in through
 * Lys. A fresh install has no grant, and these still show who holds the
 * directory and what signs in through it.
 */
import type { GrantWorld } from '../grants/model';
import { send } from '../apps/SchemaBuilder';
import type { AppRecord } from '../apps/Apps';

/** One connection the install records: who, to what, and in which words. */
export interface Installed {
  holder: string;
  name: string;
  target: string;
  words: string;
}

/**
 * The installed connections the caller may see. Only the administrator is
 * answered the whole directory, so only the administrator's view names them.
 */
export async function installedOf(world: GrantWorld): Promise<Installed[]> {
  if (world.people.scope !== 'directory') return [];
  const me = world.me.person;
  const administrator: Installed = { holder: me.id, name: me.display_name, target: 'directory', words: 'administers' };
  const { apps } = await send<{ apps: AppRecord[] }>('GET', '/apps');
  const registered = apps
    .filter((app) => app.state !== 'retired')
    .map((app): Installed => ({ holder: 'app:' + app.id, name: app.name, target: 'directory', words: 'signs in through Lys, ' + app.state }));
  return [administrator, ...registered];
}
