/** One read of the computers for the parts of a page that draw together. A page provides it; a part with no such page around it reads for itself. */
import { createContext } from 'react';
import { request } from '../../api';
import type { NetworkView } from './contract';

const read = (): Promise<NetworkView> => request<NetworkView>('/network');

export const NetworkRead = createContext<() => Promise<NetworkView>>(read);

/** A reader that asks once and gives every caller that same answer. */
export function oneNetworkRead(): () => Promise<NetworkView> {
  let held: Promise<NetworkView> | null = null;
  return () => (held ??= read());
}
