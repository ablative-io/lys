/** The tool-boundary policy and refusal shapes the identity service answers for one agent. */
export type RuleKind = 'tool' | 'path_prefix' | 'host';
export type NamedResource = { kind: string; id: string };
export type Authority = 'hard' | { permission: { resource: NamedResource; action: string } };
export type Rule = { id: string; tool: string; kind: RuleKind; target?: string; authority: Authority };
export type Policy = { version: number; agent: string; rules: Rule[] };
export type PolicyView = { agent: string; policy: Policy | null; digest: string | null; applies: string };
export type Needed = { rule: string; resource: NamedResource; action: string };
export type RefusalRecord = {
  version: number; source: string; attempt: string; session: string; agent: string; at: number; tool: string; target: string;
  policy_version: number; rule: string | null; check: string; grantable: boolean; permission: Needed | null; grantor: string | null; words: string;
};
export type RefusalsView = { agent: string; refusals: RefusalRecord[]; read_from: string[] };

export const KINDS: Record<RuleKind, string> = { tool: 'The whole tool', path_prefix: 'A path and everything under it', host: 'A web host' };
