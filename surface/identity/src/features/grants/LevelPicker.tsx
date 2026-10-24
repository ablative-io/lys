/**
 * "They may", wherever a person chooses what someone may do: three levels read in a breath (view, edit, own), each
 * said from the actions the model gives it and never from its name, so a model that changes what a level carries
 * changes its words with it. Every other relation folds under "More", grouped by area, one sentence each. An app's
 * kind, whose few relations are its own, lists them by name, each said by the model's sentences when it has one for
 * every action it carries. Agents keep the per-action picker: an agent is never given a level.
 */
import { useId } from 'react';
import type { ReactNode } from 'react';
import type { GrantModel } from '../../generated/grants';
import './grants.css';

/** The three levels, narrowest first, by the relation names the shipped model gives them. */
const LEVELS: readonly (readonly [string, string])[] = [['viewer', 'View it'], ['editor', 'Edit it'], ['owner', 'Own it']];
export const LEVEL_NAMES: readonly string[] = LEVELS.map(([relation]) => relation);

/** The actions a level's words name; any other it carries is counted, so a wide level never reads as a narrow one. */
const NAMED = new Set(['view', 'read', 'edit', 'grant']);

/** What a level lets them do, from its actions: seeing, changing, giving others access, and how many other actions besides. */
export function levelWords(actions: readonly string[]): string {
  const others = actions.filter((action) => !NAMED.has(action)).length;
  const parts = [actions.includes('view') || actions.includes('read') ? 'see it' : '', actions.includes('edit') ? 'change it' : '', actions.includes('grant') ? 'give others access to it' : '',
    others ? others + (others === 1 ? ' other action' : ' other actions') : ''].filter(Boolean);
  if (!parts.length) return 'Nothing.';
  const said = parts.length === 1 ? parts[0] : parts.slice(0, -1).join(', ') + ' and ' + parts[parts.length - 1];
  return said.charAt(0).toUpperCase() + said.slice(1) + '.';
}

/** A relation outside the levels, in one sentence: a single action by the model's sentence for it, anything wider by its actions. */
const oneWords = (model: GrantModel, actions: string[]): string =>
  actions.length === 1 ? model.action_sentences[actions[0]] ?? actions[0] : actions.map((action) => model.action_sentences[action] ?? action).join('; ');

/** The area a relation outside the levels is filed under: the part of its one action before the first dot. */
const areaOf = (actions: string[]): string => {
  const head = actions.length === 1 ? actions[0].split('.')[0] : 'Several actions';
  return head.charAt(0).toUpperCase() + head.slice(1);
};

export function LevelPicker({ model, relations, value, onChange, name, legend = 'They may', allowed = () => true, refusedWords = 'More than you hold', disabled = false, more, moreOpen = false, moreLabel = 'More: one specific thing they may do' }: {
  model: GrantModel; relations: Record<string, string[]>; value: string; onChange: (relation: string) => void;
  /** The form field the chosen relation is sent under. */
  name?: string; legend?: string;
  /** Whether a relation may be chosen here; one that may not is shown greyed with `refusedWords`. */
  allowed?: (actions: string[]) => boolean; refusedWords?: string; disabled?: boolean;
  /** What "More" holds instead of the one-action select, for a form that asks for several actions at once. */
  more?: ReactNode;
  /** Opens More because what it holds is already chosen. */
  moreOpen?: boolean;
  /** The fold's own words, so a person asking for themselves is not called "they". */
  moreLabel?: string;
}) {
  const group = useId();
  // The levels are offered only where the model has all three; an app's kind, with its own few relations, lists them by name.
  const app = !LEVELS.every(([relation]) => relation in relations);
  const levels = app ? [] : LEVELS;
  const shown: [string, string, string][] = app
    ? Object.entries(relations).map(([relation, actions]) => [relation, relation, actions.every((action) => action in model.action_sentences) ? oneWords(model, actions) : actions.join(', ')])
    : levels.map(([relation, label]) => [relation, label, levelWords(relations[relation])]);
  const others = app ? [] : Object.entries(relations).filter(([relation]) => !LEVEL_NAMES.includes(relation));
  const areas = [...new Set(others.map(([, actions]) => areaOf(actions)))].sort();
  const pickedOther = others.some(([relation]) => relation === value);
  return <fieldset className="level-picker" disabled={disabled}>
    <legend>{legend}</legend>
    {name ? <input type="hidden" name={name} value={value} /> : null}
    {shown.map(([relation, label, words]) => {
      const may = allowed(relations[relation]);
      return <label key={relation} className={'level' + (may ? '' : ' level-refused')}>
        <input type="radio" name={group} value={relation} checked={value === relation} disabled={!may} onChange={() => onChange(relation)} />
        <span><b>{label}</b> <span className="sec">{may ? words : refusedWords}</span></span>
      </label>;
    })}
    {others.length || more ? <details className="level-more" open={pickedOther || moreOpen || undefined}>
      <summary>{moreLabel}</summary>
      {more ?? <select aria-label="One specific thing they may do" value={pickedOther ? value : ''} onChange={(event) => onChange(event.target.value)}>
        <option value="">Choose one</option>
        {areas.map((area) => <optgroup key={area} label={area}>
          {others.filter(([, actions]) => areaOf(actions) === area).map(([relation, actions]) =>
            <option key={relation} value={relation} disabled={!allowed(actions)}>{oneWords(model, actions)}{allowed(actions) ? '' : ' (' + refusedWords.toLowerCase() + ')'}</option>)}
        </optgroup>)}
      </select>}
    </details> : null}
  </fieldset>;
}
