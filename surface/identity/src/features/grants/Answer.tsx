import type { Grant } from '../../generated/grants';
import type { Answer } from './check';
import { Chain } from './Chain';
import { chainOf, named, namesTitle, needsText } from './model';
import type { GrantWorld } from './model';

/** The answer as the mock-up's answerHtml draws it: the verdict, the path to a person or the named reason, and what it was judged under. */
export function AnswerView({ w, a, land }: { w: GrantWorld; a: Answer; land: boolean }) {
  const exercised = a.ok ? w.byId.get(a.permit.grant) : undefined;
  const chain: Grant[] = exercised ? chainOf(w, exercised) : [];
  return (
    <>
      {a.ok ? (
        <>
          <div className="answer">
            <span className={'verdict-mark yes' + (land ? ' land' : '')}>Yes</span>
            <span className="why">{needsText(w, a.action, exercised?.relation ?? null)}.</span>
          </div>
          <div style={{ marginTop: 8 }}>{chain.length ? <Chain w={w} chain={chain} /> : <span className="dim" title={namesTitle(w, a.permit.path.join(' '))}>{a.permit.path.map((step) => named(w, step)).join(' → ')}</span>}</div>
        </>
      ) : (
        <div className="answer">
          <span className={'verdict-mark no' + (land ? ' land' : '')}>No</span>
          <span className="tag">{a.kind}</span>
          <span className="why" title={namesTitle(w, a.why)}>{named(w, a.why)}.</span>
        </div>
      )}
      <div className="meta-line">
        {a.ok ? (
          <span>model v{a.permit.model_version}</span>
        ) : (
          <span>model v{w.model.version}</span>
        )}
        {a.ok ? <span>change {a.permit.revision}</span> : a.revision !== null ? <span>change {a.revision}</span> : null}
        <span>checked {a.t}</span>
        <span><span className="svc built-in">built in</span> asks before acting</span>
      </div>
    </>
  );
}
