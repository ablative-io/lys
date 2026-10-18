import { EffectiveSettings } from './EffectiveSettings';
import type { ReactNode } from 'react';
import { useParams } from 'react-router';
import { request, useLoad } from '../../api';
import { PageHeader } from '../../shell/PageHeader';
import { useShell } from '../../shell/ShellContext';
import { ConfigTabs } from './ConfigTabs';
import { AccountSettings } from '../people/Account';
import './settings.css';
import { AdministratorContact, useAdministratorContact } from '../signin/AccountHelp';

function Row({ t, d, children }: { t: string; d: string; children: ReactNode }) {
  return (
    <div className="set-row">
      <div>
        <div>{t}</div>
        <div className="d">{d}</div>
      </div>
      {children}
    </div>
  );
}

/** How Lys looks in this browser: kept by the browser, so it is a person's own and never needs a permission. */
function Layout() {
  const shell = useShell();
  return (
    <section aria-label="Layout" className="settings-layout">
      <h2>Layout</h2>
      <Row t="Menu side" d="Which edge of the screen the menu and Help sit on, in this browser.">
        <div className="seg">
          <button type="button" data-dock="left" className={shell.dockRight ? '' : 'on'} aria-pressed={!shell.dockRight} onClick={() => shell.setDockSide('left')}>
            Left
          </button>
          <button type="button" data-dock="right" className={shell.dockRight ? 'on' : ''} aria-pressed={shell.dockRight} onClick={() => shell.setDockSide('right')}>
            Right
          </button>
        </div>
      </Row>
      <Row t="Menu labels" d="Show the name beside every icon in the menu, in this browser.">
        <div className="seg">
          <button type="button" data-labels="icons" className={shell.labels ? '' : 'on'} aria-pressed={!shell.labels} onClick={() => shell.setLabels(false)}>
            Icons
          </button>
          <button type="button" data-labels="labels" className={shell.labels ? 'on' : ''} aria-pressed={shell.labels} onClick={() => shell.setLabels(true)}>
            Labels
          </button>
        </div>
      </Row>
    </section>
  );
}

/**
 * A person who signs in with Google has no Lys password, and both changes above ask for the current one. The
 * service does not yet say which case a person is in (`/me/account` names only email and enabled; CONSENSUS 05
 * item 9), so the act that works today, an administrator setting a password, is said as a condition.
 */
function NoPasswordYet() {
  const { administrator } = useAdministratorContact();
  // The administrator sets another person's password on their file (PersonSecurity's PersonAccount), never their own.
  const service = useLoad(() => request('/configuration'), 'settings-tabs-configuration');
  const next = service.status === 'loading' ? null : service.status === 'ok'
    ? <>You administer this Lys, so there is no one to ask: until Lys can set a first password here, keep signing in with Google.</>
    : <>Ask <AdministratorContact administrator={administrator} /> to set one for you, then change it here.</>;
  return <p className="note settings-no-password">Signed in with Google and never set a Lys password? Changing your email or password here asks for one you do not have. {next}</p>;
}

/**
 * Settings, split by whose each setting is. "For you" (`/settings/account`, and `/settings/layout`) is the
 * person's own sign-in and layout, open to everyone: the navigation never gates that address. "This service"
 * (`/settings` and its sections) is the startup configuration, which only administrators may read.
 */
export function Settings() {
  const { sec } = useParams();
  const personal = sec === 'account' || sec === 'layout';
  return (
    <div className="page fill">
      <PageHeader title="Settings" subtitle={personal ? 'Your sign-in, and how Lys looks in this browser.' : 'The settings this Lys started with, shown to its administrators.'} />
      <ConfigTabs on={personal ? 'account' : 'settings'} />
      <div className="card pane" data-section={sec}>
        {personal ? (
          <>
            <AccountSettings />
            <NoPasswordYet />
            <Layout />
          </>
        ) : (
          <EffectiveSettings />
        )}
      </div>
    </div>
  );
}
