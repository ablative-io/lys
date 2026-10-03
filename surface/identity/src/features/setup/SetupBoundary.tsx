/** Offer setup only when the service identifies the verified administrator as unbound. */
import { useState } from 'react';
import type { ReactNode } from 'react';
import { api, useLoad } from '../../api';
import { Loading } from '../signin/Gate';
import { Setup } from './Setup';

export function SetupBoundary({ children }: { children: ReactNode }) {
  const [revision, setRevision] = useState(0);
  const me = useLoad(api.me, String(revision));
  if (me.status === 'loading') return <Loading />;
  if (me.status === 'refused' && me.refused.refusal.refusal === 'SetupRequired') {
    return <Setup completed={() => setRevision((value) => value + 1)} />;
  }
  return children;
}
