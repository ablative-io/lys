import type { GrantModel, ResourceRef } from '../../generated/grants';
import { actionWords } from './action-words';

export interface ActionChoice { id: string; resource: ResourceRef; relation: string; actions: string[]; reason: string }
export interface ActionGroup { id: string; title: string; choices: ActionChoice[]; everything?: ActionChoice }

export function withheldFromAgents(model: GrantModel | null): string[] | null {
  if (!model || !('withheld_from_agents' in model) || !Array.isArray(model.withheld_from_agents)
    || model.withheld_from_agents.some((action) => typeof action !== 'string')) return null;
  return model.withheld_from_agents as string[];
}

const carriers = new WeakMap<GrantModel, Map<string, string>>();
/** Only a relation carrying exactly one act can represent a single tick. */
export function singleActionCarriers(model: GrantModel): Map<string, string> {
  let index = carriers.get(model);
  if (!index) {
    index = new Map();
    for (const [relation, actions] of Object.entries(model.relations).sort(([a], [b]) => a.localeCompare(b))) {
      if (actions.length !== 1) continue;
      const action = actions[0];
      if (!index.has(action) || relation === 'only.' + action) index.set(action, relation);
    }
    carriers.set(model, index);
  }
  return index;
}

/** Selecting everything selects the offered acts, never a broader agent relation. */
export function ActionPicker({ model, groups, selected, change, disabled = false, agents = true }: {
  model: GrantModel; groups: ActionGroup[]; selected: string[]; change: (selected: string[]) => void; disabled?: boolean; agents?: boolean;
}) {
  const withheld = withheldFromAgents(model);
  if (agents && withheld === null) return <p>Agent access choices are unavailable until Lys declares which actions are withheld from agents. Nothing is selected.</p>;
  const excluded = new Set(withheld ?? []);
  const held = new Set(selected);
  const toggle = (ids: string[], checked: boolean) => {
    const next = new Set(selected);
    for (const id of ids) { if (checked) next.add(id); else next.delete(id); }
    change([...next]);
  };
  return <>{groups.map((group) => {
    const choices = group.choices.filter((choice) => !agents || choice.resource.kind.includes('.') || choice.actions.every((action) => !excluded.has(action)));
    if (!choices.length) return null;
    const available = choices.filter((choice) => !choice.reason);
    const everything = agents ? undefined : group.everything;
    const all = everything ? [everything.id] : available.map((choice) => choice.id);
    return <fieldset key={group.id} disabled={disabled} style={{ border: 0, padding: 0 }}>
      <legend>{group.title}</legend>
      {available.length ? <label><input type="checkbox" name="all_actions" checked={all.every((id) => held.has(id))}
        onChange={(event) => change(event.target.checked ? [...selected.filter((id) => !choices.some((choice) => choice.id === id)), ...all] : selected.filter((id) => !all.includes(id)))} /> Everything here</label> : null}
      {choices.map((choice) => <div key={choice.id}><label className={choice.reason ? 'dim' : undefined}>
        <input type="checkbox" name="action" value={choice.id} disabled={Boolean(choice.reason)} checked={held.has(choice.id)}
          onChange={(event) => { if (everything) change(event.target.checked ? [...selected.filter((id) => id !== everything.id), choice.id] : selected.filter((id) => id !== choice.id)); else toggle([choice.id], event.target.checked); }} /> {actionWords(model, choice.resource, choice.actions)}
      </label>{choice.reason ? <p className="hint">{choice.reason}</p> : null}</div>)}
    </fieldset>;
  })}</>;
}
