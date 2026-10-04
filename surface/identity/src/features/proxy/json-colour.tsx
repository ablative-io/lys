/** JSON text set out in colour. Nothing is added or left out: the text read back is the text given. */
import type { ReactNode } from 'react';

/** A quoted word (a name when a colon follows it), one of true, false and null, or a number. */
const PART = /("(?:\\.|[^"\\])*")(\s*:)?|\b(true|false|null)\b|(-?\d+(?:\.\d+)?(?:[eE][+-]?\d+)?)/g;

export function Coloured({ text }: { text: string }) {
  const parts: ReactNode[] = [];
  let last = 0;
  for (const found of text.matchAll(PART)) {
    const at = found.index;
    const [whole, word, colon, plain] = found;
    if (at > last) parts.push(text.slice(last, at));
    if (word === undefined) parts.push(<span key={at} className={plain === undefined ? 'json-number' : 'json-plain'}>{whole}</span>);
    else parts.push(<span key={at} className={colon ? 'json-name' : 'json-word'}>{word}</span>, colon ?? '');
    last = at + whole.length;
  }
  parts.push(text.slice(last));
  return <>{parts}</>;
}
