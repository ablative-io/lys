/** The one control every act is pressed through: a symbol, and one short word beside it where the symbol alone
 *  would not be known (Tom, 5 October 2026: "get rid of everything that's written in words, replace it with a
 *  symbol where we can"). What a screen reader says, and what hovering shows, is always the whole act. */
import type { ButtonHTMLAttributes } from 'react';
import { SYMBOLS } from './symbols';
import type { SymbolName } from './symbols';

type Given = Omit<ButtonHTMLAttributes<HTMLButtonElement>, 'aria-label' | 'title' | 'children' | 'type'>;

export type ActProps = Given & {
  symbol: SymbolName;
  /** The whole act, as a sentence would say it: "Retire Waffles the Terrible". */
  name: string;
  /** What hovering shows, where it says more than the name; left out, hovering shows the name. */
  hint?: string;
  /** One short word drawn beside the symbol. Left out, the symbol stands alone. */
  word?: string;
  tone?: 'plain' | 'primary' | 'danger';
  /** A form's own act is `submit`; every other act is a plain button and never submits by accident. */
  type?: 'button' | 'submit';
};

export function Act({ symbol, name, hint, word, tone = 'plain', type = 'button', className, ...rest }: ActProps) {
  const classes = ['act', tone === 'plain' ? '' : tone, word ? 'worded' : '', className ?? ''].filter(Boolean).join(' ');
  return (
    <button {...rest} type={type} className={classes} aria-label={name} title={hint ?? name} data-symbol={symbol}>
      <svg viewBox="0 0 24 24" aria-hidden="true" focusable="false">{SYMBOLS[symbol]}</svg>
      {word ? <span className="act-word">{word}</span> : null}
    </button>
  );
}
