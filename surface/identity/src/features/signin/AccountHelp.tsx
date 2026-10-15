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

/** Both sign-in entries read the same public contact answer. */
export function AccountHelp() {
  const help = useAdministratorContact();
  return <AccountHelpWords help={help} />;
}

export function AccountHelpWords({ help }: { help: Help }) {
  const who = administratorWords(help.administrator);
  return <p className="note account-help">No account? Ask {who} to admit you. Contact {who} for sign-in help.
    {help.failure ? <span role="status"> {help.failure}</span> : null}
  </p>;
}
