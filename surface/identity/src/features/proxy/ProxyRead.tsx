/** A call read: what went in, then what came out, in words; the JSON of anything that is not words, in colour. */
import { Coloured } from './json-colour';
import { readIn, readOut } from './proxy-read';
import type { Piece } from './proxy-read';

const chars = (pieces: Piece[]): number => pieces.reduce((sum, each) => sum + (each.text?.length ?? 0), 0);
const many = (count: number, one: string): string => count.toLocaleString('en-AU') + ' ' + one + (count === 1 ? '' : 's');
const Json = ({ value }: { value: unknown }) => <pre className="proxy-json"><Coloured text={JSON.stringify(value, null, 2) ?? 'null'} /></pre>;

function Shown({ piece }: { piece: Piece }) {
  if (piece.kind === 'text') return piece.text === null ? <Json value={piece.json} /> : <p className="proxy-words">{piece.text}</p>;
  if (piece.kind === 'thinking') return <p className="proxy-words dim" data-piece="thinking">{piece.text ? piece.text : 'Thinking, which the provider did not show.'}</p>;
  if (piece.kind === 'tool') return <div className="proxy-piece" data-piece="tool"><b>Tool asked for: {piece.label}</b><Json value={piece.json} /></div>;
  if (piece.kind === 'result') return <div className="proxy-piece" data-piece="result"><b>{piece.label}</b>{piece.text === null ? <Json value={piece.json} /> : <p className="proxy-words">{piece.text}</p>}</div>;
  return <div className="proxy-piece" data-piece="other"><b>{piece.label}</b><Json value={piece.json} /></div>;
}

/** What a turn holds, said in a few words for its heading. */
const holds = (pieces: Piece[]): string => {
  const [tools, results] = [pieces.filter((each) => each.kind === 'tool'), pieces.filter((each) => each.kind === 'result')];
  return [chars(pieces) ? many(chars(pieces), 'character') : '', tools.length ? 'asks for ' + tools.map((each) => each.label).join(', ') : '', results.length ? many(results.length, 'tool result') : ''].filter(Boolean).join(' · ');
};

export function ProxyRead({ request, response, unreadable }: { request: unknown; response: unknown; unreadable: { request: string | null; response: string | null } }) {
  const [went, came] = [readIn(request), readOut(response)];
  // The last turn is open, and so is the person's last, which a harness often follows with a turn of its own.
  const asked = went ? went.turns.map((each) => each.role).lastIndexOf('user') : -1;
  return <div className="proxy-read">
    <section data-read="in" aria-label="What went in"><h3>What went in</h3>
      {went ? <>
        {went.settings.length ? <p className="proxy-call-line">{went.settings.map(([name, value]) => name + ': ' + value).join(' · ')}</p> : null}
        {went.system.length ? <details data-part="system"><summary>System prompt <span className="dim">{many(went.system.length, 'part')} · {many(chars(went.system), 'character')}</span></summary>{went.system.map((each, index) => <Shown key={index} piece={each} />)}</details> : null}
        {went.tools.length ? <details data-part="tools"><summary>Tools offered <span className="dim">{went.tools.length.toLocaleString('en-AU')}</span></summary><p className="proxy-words">{went.tools.join(', ')}</p></details> : null}
        {went.turns.map((turn, index) => <details key={index} data-part="turn" data-role={turn.role} open={index === went.turns.length - 1 || index === asked}>
          <summary>{index + 1}. {turn.role} <span className="dim">{holds(turn.pieces)}</span></summary>{turn.pieces.map((each, at) => <Shown key={at} piece={each} />)}</details>)}
      </> : <p className="dim" role="status">{request === null || request === undefined ? 'This call has no request that can be read as JSON.' + (unreadable.request ? ' ' + unreadable.request : '') : 'This request is not in a shape this view sets out. It is whole under Request.'}</p>}
    </section>
    <section data-read="out" aria-label="What came out"><h3>What came out</h3>
      {came ? <>
        {came.error ? <p className="why-not" role="status" data-part="error">The provider answered an error: {came.error.type}{came.error.message ? ', "' + came.error.message + '"' : ''}.</p> : null}
        {came.pieces.map((each, index) => <Shown key={index} piece={each} />)}
        {came.stop ? <p className="proxy-call-line" data-part="stop">Stopped: {came.stop}</p> : null}
        {!came.error && !came.pieces.length ? <p className="dim">The response holds no content.</p> : null}
      </> : <p className="dim" role="status">{response === null || response === undefined ? 'This call has no response that can be read as JSON.' + (unreadable.response ? ' ' + unreadable.response : '') : 'This response is not in a shape this view sets out. It is whole under Response.'}</p>}
    </section>
  </div>;
}
