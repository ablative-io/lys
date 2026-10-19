import type { Seed } from './types';

/** Apps and Settings: what signs in with Lys, and how this Lys runs. */
export const APPS: Seed[] = [
  {
    id: 'apps.apps',
    title: 'Apps',
    summary: 'Every app that signs in with Lys and has its permissions checked here.',
    body: [
      'Apps has its own place in the menu. Connect an app lists the apps Lys knows how to connect, then Another app for any app that connects to Lys by its web address, each with Connect. Each app shows its state; nothing an app registers takes effect until you approve it here.',
      'Approving an app approves the permissions it asks for and the return addresses its sign-in may send a person back to. Declining sends you back to the app, which is told no; nothing it asked for takes effect.',
      'Add your own app (for developers) registers an app by name and id and builds its permission template.',
    ],
    steps: [{ text: 'Open Apps.', href: '#/apps', link: 'Open Apps' }],
    related: ['start.app', 'apps.changes', 'apps.configuration'],
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
    title: 'Settings',
    summary: 'How this Lys runs, the services it uses and the accounts people sign in with.',
    body: [
      'Settings (g s) has four tabs. Your account changes your sign-in email and password. This service holds the layout: which side the menu and Help sit on, and whether the menu shows labels, with the settings in effect below. Services and sign-in providers lists the services this Lys is configured to use and the sign-in providers people may use. Apps is not a tab here: it has its own place in the menu.',
      'Configured means a service is selected in this installation. It does not confirm that service is reachable now.',
    ],
    steps: [
      { text: 'Open Settings.', href: '#/settings', link: 'Open Settings' },
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
