import { kindOf } from '../../generated';

/** A file's sections; keys 1 to 7 reach them in this order. An agent has all seven; a person has no Settings. */
export const TABS: [string, string][] = [
  ['profile', 'Role'], ['provisioning', 'Settings'], ['access', 'Access'], ['budgets', 'Limits and goals'],
  ['sessions', 'Sessions'], ['credentials', 'Credentials'], ['record', 'Record'],
];

/** The sections of one file, by whose file it is. */
export const tabsFor = (id: string): [string, string][] => (kindOf(id) === 'agent' ? TABS : TABS.filter(([key]) => key !== 'provisioning'));

/** Addresses that were sections of their own and are now part of another: rules under Settings, memory under Sessions, certificates under Credentials. */
export const MERGED: Record<string, string> = { policy: 'provisioning', memory: 'sessions', certificate: 'credentials' };

/** What each lifecycle state may move to, as the directory's transition table allows. */
export const ACTIONS: Record<string, string[]> = {
  registered: ['activate'],
  active: ['suspend', 'retire'],
  suspended: ['reinstate', 'retire'],
  retired: [],
};
