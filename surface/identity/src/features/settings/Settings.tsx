import { EffectiveSettings } from './EffectiveSettings';
import type { ReactNode } from 'react';
import { useParams } from 'react-router';
import { useShell } from '../../shell/ShellContext';
import { ConfigTabs } from './ConfigTabs';

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

/** Layout preferences and the administrator's effective startup configuration, every section on the one page. An address that names a section still opens the page. */
export function Settings() {
  const shell = useShell();
  const { sec } = useParams();
  return (
    <div className="page fill">
      <ConfigTabs on="settings" />
      <div className="head"><div><h1>Configuration</h1>
        <p className="sub">How this service runs. It stands on its own; nothing else needs to be installed for it to be useful.</p></div></div>
      <div className="card pane" data-section={sec}>
        <h2>Layout</h2>
              <Row t="Dock side" d="Which edge the rail and its drawer occupy. The screen takes the other.">
                <div className="seg">
                  <button data-dock="left" className={shell.dockRight ? '' : 'on'} onClick={() => shell.setDockSide('left')}>Left</button>
                  <button data-dock="right" className={shell.dockRight ? 'on' : ''} onClick={() => shell.setDockSide('right')}>Right</button>
                </div>
              </Row>
              <Row t="Rail labels" d="Show the name beside every icon in the rail.">
                <div className="seg">
                  <button data-labels="icons" className={shell.labels ? '' : 'on'} onClick={() => shell.setLabels(false)}>Icons</button>
                  <button data-labels="labels" className={shell.labels ? 'on' : ''} onClick={() => shell.setLabels(true)}>Labels</button>
                </div>
              </Row>
        <EffectiveSettings />
      </div>
    </div>
  );
}
