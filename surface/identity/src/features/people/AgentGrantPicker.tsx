/**
 * What a new agent is given, on Add an agent: one table, grouped by resource.
 *
 * One head (Give, What it may do) for the whole table. Each resource is a group
 * row with its readable name; ticking the group row gives everything in it that
 * can be given. A reason a whole group cannot be given is said once, on the
 * group row; a row says its own reason only where it differs from the group's.
 * The choices themselves (which grants, which actions, why one cannot be given)
 * come from `grantOptions`; this only lays them out.
 */
import { useId } from 'react';
import type { GrantModel, ResourceRef } from '../../generated/grants';
import { actionWords } from '../grants/action-words';
import './add-agent.css';

export interface GrantChoice { id: string; resource: ResourceRef; actions: string[]; reason: string }
export interface GrantGroup { id: string; title: string; href?: string; choices: GrantChoice[] }

/** The one reason every choice of a group shares, or empty when any choice can be given or the reasons differ. */
export function sharedReason(choices: GrantChoice[]): string {
  const first = choices[0]?.reason ?? '';
  return first && choices.every((choice) => choice.reason === first) ? first : '';
}

export function AgentGrantPicker({ model, groups, selected, change, disabled = false }: {
  model: GrantModel; groups: GrantGroup[]; selected: string[]; change: (selected: string[]) => void; disabled?: boolean;
}) {
  const base = useId();
  const held = new Set(selected);
  const shown = groups.filter((group) => group.choices.length);
  if (!shown.length) return null;
  return <table className="usage-table agent-grants" aria-label="What this agent is given">
    <colgroup><col className="agent-grants-give" /><col /></colgroup>
    <thead><tr><th>Give</th><th>What it may do</th></tr></thead>
    {shown.map((group) => {
      const shared = sharedReason(group.choices);
      const available = group.choices.filter((choice) => !choice.reason);
      const ids = available.map((choice) => choice.id);
      const every = ids.length > 0 && ids.every((id) => held.has(id));
      const allId = base + group.id + ':all';
      // What can be given first; what cannot follows, each with its own reason unless the group says it once.
      const ordered = [...available, ...group.choices.filter((choice) => choice.reason)];
      return <tbody key={group.id} className="agent-grants-group">
        <tr className="agent-grants-resource" data-resource={group.id}>
          <td>{ids.length > 1 ? <input type="checkbox" id={allId} name="all_actions" disabled={disabled} checked={every} aria-label={'Give everything on ' + group.title}
            onChange={(event) => change(event.target.checked ? [...selected.filter((id) => !ids.includes(id)), ...ids] : selected.filter((id) => !ids.includes(id)))} /> : null}</td>
          <th scope="rowgroup">{group.href ? <a href={group.href}>{group.title}</a> : group.title}{shared ? <span className="hint">{shared}</span> : null}</th>
        </tr>
        {ordered.map((choice) => <tr key={choice.id} className={choice.reason ? 'dim' : undefined}>
          <td><input type="checkbox" id={base + choice.id} name="action" value={choice.id} disabled={disabled || Boolean(choice.reason)} checked={held.has(choice.id)}
            onChange={(event) => change(event.target.checked ? [...selected.filter((id) => id !== choice.id), choice.id] : selected.filter((id) => id !== choice.id))} /></td>
          <td><label htmlFor={base + choice.id}>{actionWords(model, choice.resource, choice.actions)}</label>{choice.reason && !shared ? <span className="hint">{choice.reason}</span> : null}</td>
        </tr>)}
      </tbody>;
    })}
  </table>;
}
