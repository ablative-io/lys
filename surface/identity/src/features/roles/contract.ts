/** Roles, immutable versions and explicitly moved holdings as the role routes serve them. */
export interface GrantTemplate { resource: { kind: string; id: string }; relation: string; days: number | null }
export interface RoleWords { responsibilities: string; goals: string; practice: string; profile: string; grant_templates: GrantTemplate[]; note: string }
export interface RoleVersion extends RoleWords { number: number; made_by: string; made_at: number }
export interface RoleHolder {
  assignment: string; holder: string; display_name: string | null; version: number; behind: boolean;
  assigned_by: string; assigned_at: number; ends_at: number | null; moves_at: null;
  state: 'holding' | 'lapsed' | 'ended'; moves: { from: number; to: number; by: string; at: number }[];
  ended_by: string | null; ended_at: number | null;
}
export interface Role { id: string; name: string; latest: number; policy: 'stays_until_moved'; versions: RoleVersion[]; holders: RoleHolder[] }
const sameTemplate = (left: GrantTemplate, right: GrantTemplate): boolean =>
  left.resource.kind === right.resource.kind && left.resource.id === right.resource.id && left.relation === right.relation && left.days === right.days;

export function sameWords(version: RoleWords, words: RoleWords): boolean {
  return version.responsibilities === words.responsibilities && version.goals === words.goals && version.practice === words.practice
    && version.profile === words.profile && version.note === words.note
    && version.grant_templates.length === words.grant_templates.length && version.grant_templates.every((template, index) => sameTemplate(template, words.grant_templates[index]));
}

const isRecord = (value: unknown): value is Record<string, unknown> => typeof value === 'object' && value !== null;

/** A grant template read back from retained bytes; null when the bytes do not carry one. */
function templateOf(value: unknown): GrantTemplate | null {
  if (!isRecord(value) || !isRecord(value.resource)) return null;
  const { kind, id } = value.resource;
  const { relation, days } = value;
  if (typeof kind !== 'string' || typeof id !== 'string' || typeof relation !== 'string' || (days !== null && typeof days !== 'number')) return null;
  return { resource: { kind, id }, relation, days };
}

/** The role words a retained request body carries; null when the bytes do not carry every word, so no answer can confirm them. */
export function roleWordsOf(body: Record<string, unknown>): RoleWords | null {
  const { responsibilities, goals, practice, profile, note, grant_templates } = body;
  if (typeof responsibilities !== 'string' || typeof goals !== 'string' || typeof practice !== 'string' || typeof profile !== 'string' || typeof note !== 'string' || !Array.isArray(grant_templates)) return null;
  const templates: GrantTemplate[] = [];
  for (const value of grant_templates) {
    const template = templateOf(value);
    if (template === null) return null;
    templates.push(template);
  }
  return { responsibilities, goals, practice, profile, note, grant_templates: templates };
}

export type RoleAnswer = Role | { role: string; holder: RoleHolder; from: RoleVersion; to: RoleVersion };
