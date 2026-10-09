/**
 * The one permissions matrix: what each relation may do, a row for each action
 * in plain words and a column for each relation, a tick where the relation
 * carries the action. It is used for every app's schema on Configuration > Apps
 * and for the permission model on Access > Model; there is no second table.
 *
 * A schema with many relations that each carry a single action (Lys's own
 * `only.…` relations) would be a matrix of one tick per column, scrolled
 * sideways for screens. Those relations are named in one column beside the
 * action they carry instead, so the matrix fits the width and loses nothing.
 */
import './schema-matrix.css';

/** A kind of a schema; `roles` are its named bundles of actions (ACCESS-004 R1), listed under its matrix. */
export interface MatrixKind { actions?: string[]; relations?: Record<string, string[]>; parents?: string[]; roles?: Record<string, string[]> }

/** More relations than this, and the single-action ones are named beside their action instead of each taking a column. */
const COLUMNS = 8;

const lower = (words: string) => words.charAt(0).toLowerCase() + words.slice(1);

export function SchemaMatrix({ kinds, label, sentence }: { kinds: [string, MatrixKind][]; label: string; sentence?: (action: string) => string | undefined }) {
  const say = (action: string) => sentence?.(action) ?? action;
  return <div className="schema-tables" aria-label={label}>
    {kinds.map(([kind, body]) => {
      const relations = Object.entries(body.relations ?? {});
      // Every action any relation carries has a row: the kind's declared actions first, then any a relation carries that the kind did not list, so nothing a relation gives is left off the screen.
      const carried = [...new Set(relations.flatMap(([, may]) => may))];
      const actions = [...(body.actions ?? []), ...carried.filter((action) => !(body.actions ?? []).includes(action))];
      const fold = relations.length > COLUMNS;
      const alone = fold ? relations.filter(([, may]) => may.length === 1) : [];
      const columns = fold ? relations.filter(([, may]) => may.length !== 1) : relations;
      const width = 1 + columns.length + (alone.length ? 1 : 0);
      return <table className="schema-table schema-matrix" key={kind} aria-label={'Relations of ' + kind}>
        <thead><tr>
          <th scope="col">{kind}</th>
          {columns.map(([relation]) => <th scope="col" key={relation}>{relation}</th>)}
          {alone.length ? <th scope="col" className="alone">The relation for this alone</th> : null}
        </tr></thead>
        <tbody>
          {relations.length ? actions.map((action) => {
            const own = alone.filter(([, may]) => may[0] === action).map(([relation]) => relation);
            return <tr key={action}>
              <th scope="row" title={say(action) === action ? undefined : action}>{say(action)}</th>
              {columns.map(([relation, may]) => <td key={relation} aria-label={relation + (may.includes(action) ? ' may ' : ' may not ') + lower(say(action))}>{may.includes(action) ? '✓' : ''}</td>)}
              {alone.length ? <td className="alone">{own.join(', ')}</td> : null}
            </tr>;
          }) : <tr><td colSpan={width} className="dim">It has no relation of its own.</td></tr>}
        </tbody>
        {body.parents?.length || Object.keys(body.roles ?? {}).length ? <tfoot>
          {body.parents?.length ? <tr><td colSpan={width} className="note">What is held on {body.parents.join(' or ')} reaches it.</td></tr> : null}
          {Object.entries(body.roles ?? {}).map(([role, may]) => <tr key={'role:' + role} className="role"><td colSpan={width} className="note" aria-label={'Role ' + role}>The role <b>{role}</b> carries {may.map(say).join(', ')}.</td></tr>)}
        </tfoot> : null}
      </table>;
    })}
    {kinds.length ? null : <p className="dim">It declares no kind.</p>}
  </div>;
}
