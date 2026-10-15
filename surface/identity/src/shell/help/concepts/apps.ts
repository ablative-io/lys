import type { Seed } from './types';

/** Apps and Configuration: what signs in with Lys, and how this Lys runs. */
export const APPS: Seed[] = [
  {
    id: 'apps.apps',
    title: 'Apps',
    summary: 'Every app that signs in with Lys and has its permissions checked here.',
    body: [
      'Apps sits under Configuration. Known apps, Cambium among them, are listed with Connect. Each app shows its state; nothing an app registers takes effect until you approve it here.',
      'Approving an app approves the permissions it asks for and the return addresses its sign-in may send a person back to. Declining sends you back to the app, which is told no; nothing it asked for takes effect.',
      'Add your own app (for developers) registers an app by name and id and builds its permission template.',
    ],
    steps: [{ text: 'Open Apps.', href: '#/apps', link: 'Open Apps' }],
    related: ['start.cambium', 'apps.changes', 'apps.configuration'],
  },
  {
    id: 'apps.changes',
    title: 'An app’s changes',
    summary: 'When an app asks for different permissions, you approve the change.',
    body: ['An approved app that later asks for more, or different, permissions shows the change on its card; it takes effect only when you approve the change.'],
    steps: [],
    related: ['apps.apps'],
  },
  {
    id: 'apps.configuration',
    title: 'Configuration',
    summary: 'How this Lys runs, the services it uses and the accounts people sign in with.',
    body: [
      'Configuration (g s) has three tabs. This service holds the layout: which side the menu and Help sit on, and whether the menu shows labels, with the settings in effect below. Services and sign-in providers lists the services this Lys is configured to use and the sign-in providers people may use. Apps lists the apps.',
      'Configured means a service is selected in this installation. It does not confirm that service is reachable now.',
    ],
    steps: [
      { text: 'Open Configuration.', href: '#/settings', link: 'Open Configuration' },
      { text: 'Add a sign-in provider under Services and sign-in providers.', href: '#/connections', link: 'Open sign-in providers' },
    ],
    related: ['start.google', 'apps.apps'],
  },
  {
    id: 'apps.svc',
    title: 'Where a type comes from',
    summary: 'Built in, or added by a connected product.',
    body: ['This service has its own types and works with nothing else installed. Other products, ours or not, add theirs when a person approves them.'],
    steps: [],
    related: ['apps.apps', 'access.grant'],
  },
];
