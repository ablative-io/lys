import { useEffect, useRef, useState } from 'react';
import { Act } from '../Act';
import { keyable } from '../keyable';
import { useShell } from '../ShellContext';
import { CONCEPTS, TRACKS, conceptById, conceptGroups } from './concepts';
import type { Concept } from './concepts';

/** One row of the index: the concept's name and its one sentence. Enter or a click opens its sheet. */
function ConceptRow({ concept, brief }: { concept: Concept; brief?: boolean }) {
  const shell = useShell();
  return (
    <div className="concept" data-help={concept.id} {...keyable(() => shell.showHelp(concept.id))}>
      <div className="t">{concept.title}</div>
      {brief ? null : <div className="s">{concept.summary}</div>}
    </div>
  );
}

/**
 * Everything the help explains: every topic by track; what is on this page is the numbered layer's, one press away in the head,
 * narrowed in place by what is typed: every word typed must appear in a concept's name, sentence, explanation or steps.
 */
function TrackIndex() {
  const [query, setQuery] = useState('');
  const groups = conceptGroups(query);
  return (
    <>
      <input className="search" id="helpQ" type="search" aria-label="Search help" placeholder="Search help…" value={query} onChange={(e) => setQuery(e.target.value)} />
      <div id="helpList" style={{ marginTop: 8 }}>
        {groups.length ? groups.map((group) => (
          <section key={group.track} className="help-track" data-track={group.track} aria-label={group.label}>
            <div className="section-h">{group.label}</div>
            {group.concepts.map((c) => <ConceptRow key={c.id} concept={c} />)}
          </section>
        )) : <div className="dim" role="status">Nothing matches “{query.trim()}”.</div>}
      </div>
    </>
  );
}

/** One concept's sheet: what it is, the steps that do it, and what to read next. */
function Sheet({ concept }: { concept: Concept }) {
  const track = TRACKS.find(([id]) => id === concept.track)?.[1] ?? '';
  const related = concept.related.map(conceptById).filter((c): c is Concept => c !== undefined);
  return (
    <>
      <div className="sheet" data-sheet={concept.id}>
        <p className="help-sheet-track">{track}</p>
        <p style={{ color: 'var(--text-primary)' }}>{concept.summary}</p>
        {concept.body.map((p) => <p key={p}>{p}</p>)}
      </div>
      {concept.steps.length ? (
        <>
          <div className="section-h">Steps</div>
          <ol className="help-steps">
            {concept.steps.map((step) => (
              <li key={step.text}>{step.text}{step.href ? <> <a href={step.href}>{step.link ?? 'Open'}</a></> : null}</li>
            ))}
          </ol>
        </>
      ) : null}
      {related.length ? (
        <>
          <div className="section-h">Read next</div>
          {related.map((c) => <ConceptRow key={c.id} concept={c} brief />)}
        </>
      ) : null}
    </>
  );
}

/** The full help, beside the screen; what it shows is the address's `topic`. */
export function HelpDock() {
  const shell = useShell();
  const chosen = shell.helpSel ? conceptById(shell.helpSel) : undefined;
  const body = useRef<HTMLDivElement>(null);
  // A new sheet starts at its top, not where the last one was scrolled to.
  useEffect(() => { body.current?.scrollTo?.(0, 0); }, [chosen?.id]);
  return (
    <div className="dock-in">
      <div className="dock-head">
        <span style={{ display: 'flex', gap: 6, alignItems: 'center' }}>
          {chosen ? <Act symbol="back" name="Back to every topic" className="small quiet" data-help="" onClick={() => shell.showHelp(null)} /> : null}
          <b style={{ fontWeight: 600 }}>{chosen ? chosen.title : 'Help'}</b>
        </span>
        <span style={{ display: 'flex', gap: 4 }}>
          <Act symbol="help" name="Number the controls on this page" hint="Mark each control on this page with its number (?)" word="Number them" data-act="explain" onClick={shell.explainOn} />
          <Act symbol="close" name="Close help" className="small quiet" data-dockbtn-close="" onClick={shell.closeDock} />
        </span>
      </div>
      <div className="dock-body" ref={body}>
        {chosen ? <Sheet concept={chosen} /> : <TrackIndex />}
      </div>
      <div className="dock-foot note">{CONCEPTS.length} topics. The address links to what is open here.</div>
    </div>
  );
}
