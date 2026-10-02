import type { GrantModel, ResourceRef } from '../../generated/grants';

const definitions = new WeakMap<GrantModel, string[]>();

/** App actions retain their own names; only the shipped model supplies sentences. */
export function actionWords(model: GrantModel, resource: ResourceRef, actions: string[]): string {
  if (resource.kind.includes('.')) return actions.join('; ');
  let shipped = definitions.get(model);
  if (!shipped) {
    shipped = Object.keys(model.action_sentences);
    definitions.set(model, shipped);
  }
  const held = new Set(actions);
  if (shipped.length && shipped.every((action) => held.has(action))) return 'everything here';
  return actions.map((action) => model.action_sentences[action] ?? action).join('; ');
}

export function resourceFromText(text: string): ResourceRef {
  const at = text.indexOf(':');
  return { kind: text.slice(0, at), id: text.slice(at + 1) };
}

export function resourceWords(resource: ResourceRef): string {
  return resource.kind === 'directory' ? `the ${resource.id} directory` : `${resource.kind} ${resource.id}`;
}
