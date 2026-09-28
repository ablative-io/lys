import { useEffect, useRef, useState } from 'react';
import { Refused } from '../../api';
import type { CannotGiveAnswer, CannotGiveItem, CannotGiveReason } from '../../generated';
import type { Grant } from '../../generated/grants';
import { askCannotGive } from './cannotGiveAnswer';
import { grantNo, onText } from './model';
import type { GrantWorld } from './model';

/** What each of the six reasons says. */
const WORDS: Record<CannotGiveReason, string> = {
  sign_in_identity: 'They prove who you are. No agent can hold them.',
  above_what_you_hold: 'More than you hold.',
  lent_to_you: 'It was lent to you for your own use; you may not pass it on.',
  use_only: 'You may use it; it does not let you pass it on.',
  people_only: 'This can be passed on only to a person.',
  agents_only: 'This can be passed on only to an agent.',
};

/** The item's name. The service decides every item; this only names it from the grants already read. */
function subjectText(w: GrantWorld, source: Grant, item: CannotGiveItem): string {
  switch (item.subject) {
    case 'grant': {
      const g = w.byId.get(item.grant);
      return g ? `${g.relation} of ${onText(g)}` : grantNo(item.grant);
    }
    case 'service_account': {
      const g = w.byId.get(item.grant);
      return 'Service account ' + (g ? g.resource.id : grantNo(item.grant));
    }
    case 'relation':
      return `${item.relation} of ${onText(source)}`;
    case 'sign_in_identity':
      return 'Your sign-in identities';
  }
}

type Shown = { at: 'asking' } | { at: 'answered'; answer: CannotGiveAnswer } | { at: 'refused'; refused: Refused };

/**
 * Everything the caller cannot give the chosen recipient, each with its one
 * reason, exactly as the service answers it (conformance 2.4). It asks again
 * when the recipient changes, and an answer for a recipient no longer chosen
 * is discarded. It computes, adds, drops, merges and reorders nothing.
 */
export function CannotGiveList({ w, source, recipient }: { w: GrantWorld; source: Grant; recipient: string }) {
  const [shown, setShown] = useState<Shown>({ at: 'asking' });
  const chosen = useRef({ source: source.id, recipient });
  chosen.current = { source: source.id, recipient };

  useEffect(() => {
    if (!recipient) return;
    setShown({ at: 'asking' });
    const current = () => chosen.current.source === source.id && chosen.current.recipient === recipient;
    askCannotGive(source.id, recipient).then(
      (answer) => {
        if (current() && answer.source === chosen.current.source && answer.recipient === chosen.current.recipient) setShown({ at: 'answered', answer });
      },
      (error: unknown) => {
        if (current()) setShown({ at: 'refused', refused: error instanceof Refused ? error : new Refused(0, { refusal: 'Unanswered', reason: String(error) }) });
      },
    );
  }, [source.id, recipient]);

  if (!recipient) return <p className="note">Choose who to give it to.</p>;
  if (shown.at === 'asking') return <p className="note" aria-live="polite">Asking the service…</p>;
  if (shown.at === 'refused') {
    return (
      <div className="why-not" role="alert" data-refusal={shown.refused.refusal.refusal}>
        <b>{shown.refused.refusal.refusal}</b>
        <div className="note">{shown.refused.refusal.reason}</div>
      </div>
    );
  }
  if (shown.answer.items.length === 0) return <p className="note">Nothing: you may give this recipient everything you hold.</p>;
  return (
    <>
      {shown.answer.items.map((item, i) => (
        <div className="why-not" key={i} data-cannot-give={item.reason} data-source={item.source ? 'source' : undefined}>
          <b>{subjectText(w, source, item)}</b>
          {item.source ? <span className="pill"> the grant you are giving from</span> : null}
          <div className="note">{WORDS[item.reason]}</div>
        </div>
      ))}
    </>
  );
}
