import { useId } from 'react';
import type { GrantModel, ResourceRef } from '../../generated/grants';
import { actionWords } from './action-words';
import { grantKindsProblem } from '../../generated/grants';
import { appKind, appKindProblem } from './model';

export interface ActionChoice { id: string; resource: ResourceRef; relation: string; actions: string[]; reason: string }
export interface ActionGroup { id: string; title: string; href?: string; choices: ActionChoice[]; everything?: ActionChoice }
/** An app relation's choice id: kept apart from action ids, so a relation named like an action never stands for that action. */
export const RELATION_CHOICE = 'relation:';
export const appAgentRefusal = "Lys can't give an agent this app's actions until the app allows it.";

export function appAgentReason(model: GrantModel, kind: string): string {
  return appKindProblem(model, kind) ?? (appKind(model, kind).agents.length === 0 ? appAgentRefusal : 'The app does not allow an agent to hold these actions.');
}

export function withheldFromAgents(model: GrantModel | null): string[] | null {
  // An undeclared model offers nothing to agents, said by the picker; never a throw while a page renders.
  if (!model || grantKindsProblem(model) !== null || !('withheld_from_agents' in model) || !Array.isArray(model.withheld_from_agents)
    || model.withheld_from_agents.some((action) => typeof action !== 'string')) return null;
  return model.withheld_from_agents as string[];
}

const carriers = new WeakMap<GrantModel, Map<string, string>>();
/** Only a relation carrying exactly one act can represent a single tick. */
export function singleActionCarriers(model: GrantModel, kind?: string): Map<string, string> {
  if (kind?.includes('.')) return carriersOf(appKind(model, kind).relations);
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

function carriersOf(relations: Record<string, string[]>): Map<string, string> {
  const index = new Map<string, string>();
  for (const [relation, actions] of Object.entries(relations).sort(([a], [b]) => a.localeCompare(b))) {
    if (actions.length === 1 && (!index.has(actions[0]) || relation === 'only.' + actions[0])) index.set(actions[0], relation);
  }
  return index;
}

export function agentMayHold(model: GrantModel, kind: string, actions: string[]): boolean {
  if (kind.includes('.')) {
    const agents = appKind(model, kind).agents;
    return actions.every((action) => agents.includes(action));
  }
  const withheld = withheldFromAgents(model);
  return withheld !== null && actions.every((action) => !withheld.includes(action));
}

/** App relations are offered only with every action they actually carry. */
export function appAgentCarriers(model: GrantModel, kind: string, actions: string[], agents = true): [string, string[]][] {
  return Object.entries(appKind(model, kind).relations).filter(([, carried]) =>
    carried.length > 0 && carried.every((action) => actions.includes(action)) && (!agents || agentMayHold(model, kind, carried)));
}

/** Selecting everything selects the offered acts, never a broader agent relation. */
function GroupedActionPicker({ model, groups, selected, change, disabled = false, agents = true }: {
  model: GrantModel; groups: ActionGroup[]; selected: string[]; change: (selected: string[]) => void; disabled?: boolean; agents?: boolean;
}) {
  const base = useId();
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
    const choices = group.choices.filter((choice) => !agents || choice.resource.kind.includes('.') || choice.actions.every((action) => !excluded.has(action)))
      .map((choice) => {
        if (!agents || !choice.resource.kind.includes('.')) return choice;
        return { ...choice, reason: agentMayHold(model, choice.resource.kind, choice.actions)
          ? choice.reason === appAgentRefusal ? '' : choice.reason : appAgentReason(model, choice.resource.kind) };
      });
    if (!choices.length) return null;
    const available = choices.filter((choice) => !choice.reason);
    const everything = agents ? undefined : group.everything;
    const all = everything ? [everything.id] : available.map((choice) => choice.id);
    const choiceIds = new Set(choices.map((choice) => choice.id));
    const allIds = new Set(all);
    const refusedChoices = choices.filter((choice) => choice.reason);
    const row = (choice: ActionChoice) => <tr key={choice.id} className={choice.reason ? 'dim' : undefined}>
      <td><input type="checkbox" id={base + choice.id} name="action" value={choice.id} disabled={Boolean(choice.reason)} checked={held.has(choice.id)}
        onChange={(event) => { if (everything) change(event.target.checked ? [...selected.filter((id) => id !== everything.id), choice.id] : selected.filter((id) => id !== choice.id)); else toggle([choice.id], event.target.checked); }} /></td>
      <td><label htmlFor={base + choice.id}>{actionWords(model, choice.resource, choice.actions)}</label>{choice.reason ? <div className="hint">{choice.reason}</div> : null}</td>
    </tr>;
    return <fieldset key={group.id} disabled={disabled} style={{ border: 0, padding: 0 }}>
      <legend>{group.href ? <a href={group.href}>{group.title}</a> : group.title}</legend>
      <table className="usage-table action-table"><colgroup><col style={{ width: 56 }} /><col /></colgroup>
        <thead><tr><th>Give</th><th>What it may do</th></tr></thead>
        <tbody>
          {available.length ? <tr><td><input type="checkbox" id={base + group.id + ':all'} name="all_actions" checked={all.every((id) => held.has(id))}
            onChange={(event) => change(event.target.checked ? [...selected.filter((id) => !choiceIds.has(id)), ...all] : selected.filter((id) => !allIds.has(id)))} /></td><td><label htmlFor={base + group.id + ':all'}>Everything here</label></td></tr> : null}
          {available.map(row)}
          {/* What cannot be ticked is kept, with its reason, under what can. */}
          {refusedChoices.map(row)}
        </tbody>
      </table>
    </fieldset>;
  })}</>;
}


type GroupedProps = Parameters<typeof GroupedActionPicker>[0];
/** `title` is the picker's one heading, its legend: `Actions` unless the form names the choice itself. */
type SingleProps = { model: GrantModel; resource: ResourceRef; actions: string[]; value: string[]; onChange: (value: string[]) => void; disabled?: boolean; agents?: boolean; title?: string };

/** A single resource uses action names as its value; grouped callers retain their source keys. */
export function ActionPicker(props: SingleProps | GroupedProps) {
  if ('groups' in props) return <GroupedActionPicker {...props} />;
  const relations = singleActionCarriers(props.model, props.resource.kind);
  const choices = props.actions.flatMap((action) => {
    if (props.agents !== false && props.resource.kind.includes('.') && !agentMayHold(props.model, props.resource.kind, [action])) return [{ id: action, resource: props.resource, relation: '', actions: [action], reason: appAgentReason(props.model, props.resource.kind) }];
    const relation = relations.get(action);
    return relation ? [{ id: action, resource: props.resource, relation, actions: [action], reason: '' }] : [];
  });
  if (props.resource.kind.includes('.')) {
    for (const [relation, actions] of appAgentCarriers(props.model, props.resource.kind, props.actions, props.agents !== false)) {
      if (actions.every((action) => choices.some((choice) => !choice.reason && choice.actions.includes(action)))) continue;
      choices.push({ id: RELATION_CHOICE + relation, resource: props.resource, relation, actions, reason: '' });
    }
  }
  const represented = new Set(choices.flatMap((choice) => choice.actions));
  return <><GroupedActionPicker model={props.model} groups={[{ id: JSON.stringify(props.resource), title: props.title ?? 'Actions', choices }]}
    selected={props.value} change={props.onChange} disabled={props.disabled} agents={props.agents} />
    {props.actions.some((action) => !represented.has(action)) ? <p>Some actions cannot be selected because the model has no relation carrying only those actions.</p> : null}</>;
}
