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
export function sameWords(version: RoleWords, words: RoleWords): boolean {
  return version.responsibilities === words.responsibilities && version.goals === words.goals && version.practice === words.practice
    && version.profile === words.profile && version.note === words.note && JSON.stringify(version.grant_templates) === JSON.stringify(words.grant_templates);
}
