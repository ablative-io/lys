/** What each provider needs switched on before Lys can sign people in with it, in the order its console asks. */
export type Provider = 'google' | 'microsoft' | 'github';

export interface Step { text: string; link?: { label: string; href: string } }
export interface Guide { name: string; blurb: string; prepare: Step[]; idLabel: string; secretLabel: string }

export const GUIDES: Record<Provider, Guide> = {
  google: {
    name: 'Google',
    blurb: 'Google accounts and Google Workspace.',
    prepare: [
      { text: 'Set up the consent screen people see: an app name, your support email, and the audience (External, or Internal for a Workspace).', link: { label: 'Open Branding', href: 'https://console.cloud.google.com/auth/branding' } },
      { text: 'While the audience is in testing, add each person who will sign in as a test user, or publish the app.', link: { label: 'Open Audience', href: 'https://console.cloud.google.com/auth/audience' } },
      { text: 'Create an OAuth client of type Web application, and add the redirect address below under Authorised redirect URIs. Sign-in needs no other API switched on.', link: { label: 'Create a client', href: 'https://console.cloud.google.com/auth/clients/create' } },
    ],
    idLabel: 'Client ID',
    secretLabel: 'Client secret',
  },
  github: {
    name: 'GitHub',
    blurb: 'GitHub personal and organisation accounts.',
    prepare: [
      { text: 'Register a new OAuth app. Use this Lys address as the homepage, and the redirect address below as the authorization callback URL.', link: { label: 'New OAuth app', href: 'https://github.com/settings/applications/new' } },
      { text: 'On the app page, generate a new client secret and copy it straight away. GitHub shows it once.' },
    ],
    idLabel: 'Client ID',
    secretLabel: 'Client secret',
  },
  microsoft: {
    name: 'Microsoft',
    blurb: 'Microsoft Entra work and school accounts.',
    prepare: [
      { text: 'Register a new application. Choose Web as the platform and enter the redirect address below.', link: { label: 'New registration', href: 'https://entra.microsoft.com/#view/Microsoft_AAD_RegisteredApps/CreateApplicationBlade' } },
      { text: 'Under API permissions, add Microsoft Graph delegated permissions openid, email and profile, then grant admin consent.' },
      { text: 'Under Certificates & secrets, create a client secret and copy its value (not its ID).' },
    ],
    idLabel: 'Application (client) ID',
    secretLabel: 'Client secret value',
  },
};
