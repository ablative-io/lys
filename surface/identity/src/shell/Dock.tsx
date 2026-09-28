import { useState } from 'react';
import { CONCEPTS } from './concepts';
import type { Concept } from './concepts';
import { keyable } from './keyable';
import { useShell } from './ShellContext';

function ConceptRow({ concept, brief }: { concept: Concept; brief?: boolean }) {
  const shell = useShell();
  return (
    <div className="concept" data-help={concept.id} {...keyable(() => shell.showHelp(concept.id))}>
      <div className="t">{concept.t}</div>
      {brief ? null : <div className="s">{concept.s}</div>}
    </div>
  );
}

function HelpPanel() {
  const shell = useShell();
  const [query, setQuery] = useState('');
  const chosen = shell.helpSel ? CONCEPTS.find((c) => c.id === shell.helpSel) : undefined;
  const q = query.toLowerCase();
  const found = CONCEPTS.filter((c) => (c.t + ' ' + c.s + ' ' + c.body).toLowerCase().includes(q));
  return (
    <div className="dock-in">
      <div className="dock-head">
        <span style={{ display: 'flex', gap: 6, alignItems: 'center' }}>
          {chosen ? (
            <button className="icon-btn" data-help="" title="Back" onClick={() => shell.showHelp(null)}>
              ‹
            </button>
          ) : null}
          <b style={{ fontWeight: 600 }}>{chosen ? chosen.t : 'Help'}</b>
        </span>
        <span style={{ display: 'flex', gap: 4 }}>
          <button className="btn" data-act="explain" title="Number everything on this screen (?)" onClick={shell.explainOn}>
            Explain this screen <span className="kbd">?</span>
          </button>
          <button className="icon-btn" data-dockbtn-close="" title="Close" onClick={shell.closeDock}>
            ×
          </button>
        </span>
      </div>
      <div className="dock-body">
        {chosen ? (
          <>
            <div className="sheet">
              <p style={{ color: 'var(--text-primary)' }}>{chosen.s}</p>
              <p>{chosen.body}</p>
            </div>
            <div className="section-h">Read next</div>
            {CONCEPTS.filter((c) => c.id !== chosen.id)
              .slice(0, 3)
              .map((c) => (
                <ConceptRow key={c.id} concept={c} brief />
              ))}
          </>
        ) : (
          <>
            <input className="search" id="helpQ" placeholder="Search help…" value={query} onChange={(e) => setQuery(e.target.value)} />
            <div id="helpList" style={{ marginTop: 8 }}>
              {found.length ? found.map((c) => <ConceptRow key={c.id} concept={c} />) : <div className="dim">Nothing matches.</div>}
            </div>
          </>
        )}
      </div>
    </div>
  );
}

function AssistantPanel() {
  const shell = useShell();
  return (
    <div className="dock-in">
      <div className="dock-head">
        <b style={{ fontWeight: 600 }}>Assistant</b>
        <span style={{ display: 'flex', gap: 6, alignItems: 'center' }}>
          <button className="icon-btn" data-dockbtn-close="" title="Close" onClick={shell.closeDock}>
            ×
          </button>
        </span>
      </div>
      <div className="dock-body">
        <div className="empty-note">
          <span className="open-q">not built yet</span> The assistant is proposed (conformance 9.4): it would see the screen
          only when you opt in, and act only through its own grants and the same requests.
        </div>
      </div>
      <div className="dock-foot">
        <div className="note">Which runtime powers it is <span className="open-q">open</span></div>
      </div>
    </div>
  );
}

export function Dock() {
  const shell = useShell();
  return (
    <aside className={'dock' + (shell.dockMode ? ' open' : '')} id="dock" aria-label="Help and assistant">
      {shell.dockMode === 'help' ? <HelpPanel /> : shell.dockMode === 'assistant' ? <AssistantPanel /> : null}
    </aside>
  );
}
