/** A file's sections, in the mock-up's order; keys 1 to 7 reach the first seven. */
export const TABS: [string, string][] = [
  ['profile', 'Role'], ['access', 'Access'], ['provisioning', 'Provisioning'], ['memory', 'Memory and context'],
  ['credentials', 'Credentials'], ['sessions', 'Sessions'], ['certificate', 'Certificate'], ['policy', 'Tool policy'],
  ['record', 'Record'], ['budgets', 'Budgets and goals'],
];

/** What each lifecycle state may move to, as the directory's transition table allows. */
export const ACTIONS: Record<string, string[]> = {
  registered: ['activate'],
  active: ['suspend', 'retire'],
  suspended: ['reinstate', 'retire'],
  retired: [],
};
