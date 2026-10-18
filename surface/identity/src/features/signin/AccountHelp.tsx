import { useEffect, useState } from 'react';
import { request } from '../../api';

interface Administrator { name: string; email: string | null }
type Help = { administrator: Administrator | null; failure: string };

export function useAdministratorContact(): Help {
  const [help, setHelp] = useState<Help>({ administrator: null, failure: '' });
  useEffect(() => {
    let live = true;
    request<{ administrator: Administrator | null }>('/sign-in/help').then(
      (answer) => {
        const administrator = answer.administrator;
        if (administrator !== null && (!administrator || typeof administrator.name !== 'string'
          || (administrator.email !== null && typeof administrator.email !== 'string'))) {
          throw new Error('Lys did not give usable administrator contact details.');
        }
        if (live) setHelp({ administrator, failure: '' });
      },
    ).catch(() => { if (live) setHelp({ administrator: null, failure: 'Lys could not read the administrator’s contact details.' }); });
    return () => { live = false; };
  }, []);
  return help;
}

export function administratorWords(administrator: Administrator | null): string {
  return administrator ? administrator.name + (administrator.email ? ' (' + administrator.email + ')' : '') : 'your administrator';
}

/** The administrator named with the next step as a link, an email to them, when Lys names one. */
export function AdministratorContact({ administrator }: { administrator: Administrator | null }) {
  return administrator?.email
    ? <>{administrator.name} (<a href={'mailto:' + administrator.email}>{administrator.email}</a>)</>
    : <>{administratorWords(administrator)}</>;
}

export function AccountHelpWords({ help }: { help: Help }) {
  return <p className="note account-help">No account yet, or cannot get in? Ask <AdministratorContact administrator={help.administrator} />.
    {help.failure ? <span role="status"> {help.failure}</span> : null}
  </p>;
}
