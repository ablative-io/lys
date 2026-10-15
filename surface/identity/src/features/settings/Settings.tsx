import { EffectiveSettings } from './EffectiveSettings';
import type { ReactNode } from 'react';
import { useParams } from 'react-router';
import { useShell } from '../../shell/ShellContext';
import { ConfigTabs } from './ConfigTabs';
import { AccountSettings } from '../people/Account';

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

/** Personal account changes have their own section beside the service's configuration. */
export function Settings() {
  const shell = useShell();
  const { sec } = useParams();
  return (
    <div className="page fill">
      <div className="head">
        <div>
          <h1>Settings</h1>
          <p className="sub">{sec === 'account' ? 'Your sign-in email and password.' : 'Where the menu sits for you, and the settings this Lys started with.'}</p>
        </div>
      </div>
      <ConfigTabs on={sec === 'account' ? 'account' : 'settings'} />
      <div className="card pane" data-section={sec}>
        {sec === 'account' ? (
          <AccountSettings />
        ) : (
          <>
            <h2>Layout</h2>
            <Row t="Menu side" d="Which edge of the screen the menu and Help sit on.">
              <div className="seg">
                <button type="button" data-dock="left" className={shell.dockRight ? '' : 'on'} aria-pressed={!shell.dockRight} onClick={() => shell.setDockSide('left')}>
                  Left
                </button>
                <button type="button" data-dock="right" className={shell.dockRight ? 'on' : ''} aria-pressed={shell.dockRight} onClick={() => shell.setDockSide('right')}>
                  Right
                </button>
              </div>
            </Row>
            <Row t="Menu labels" d="Show the name beside every icon in the menu.">
              <div className="seg">
                <button type="button" data-labels="icons" className={shell.labels ? '' : 'on'} aria-pressed={!shell.labels} onClick={() => shell.setLabels(false)}>
                  Icons
                </button>
                <button type="button" data-labels="labels" className={shell.labels ? 'on' : ''} aria-pressed={shell.labels} onClick={() => shell.setLabels(true)}>
                  Labels
                </button>
              </div>
            </Row>
            <EffectiveSettings />
          </>
        )}
      </div>
    </div>
  );
}
