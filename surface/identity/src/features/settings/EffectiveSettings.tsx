/** Configuration shows the effective server values. It links nowhere the rail already goes. */
import { api, request, useLoad } from '../../api';
import { Gate } from '../signin/Gate';
import { duration } from '../file/time';
import { useState } from 'react';
import { ModelWindows } from './ModelWindows';
import type { Organisation } from './ModelWindows';

interface Configuration {
  source: 'startup_configuration'; mutable_in_browser: false;
  /** The organisation setting, and the models kept calls named that it has no window for; absent from a service that keeps neither. */
  organisation?: Organisation; models_undeclared?: string[];
  sign_in: { provider_origin: string; session_seconds: number; secure_cookie: boolean };
  directory: { roles_configured: boolean };
  permissions: { model_version: number; projection: 'local' | 'spicedb' };
  secrets: { configured: boolean };
  runtimes: { machines_configured: boolean; provisioning_configured: boolean };
  storage: { directory_format: string; grant_format: string; requests_configured: boolean };
}
const configured = (value: boolean) => value ? 'Configured' : 'Not configured';

/** Every section of the startup configuration, or the one named. */
export function EffectiveSettings({ section }: { section?: string }) {
  const show = (name: string) => section === undefined || section === name;
  // Saving the table reads the configuration again, at its new version.
  const [round, setRound] = useState(0);
  const load = useLoad(() => request<Configuration>('/configuration'), 'effective-configuration:' + round);
  const authority = useLoad(api.authority, 'service-build');
  return <Gate load={load} title="Configuration" ok={(settings) => <>
    <p className="note">These are the settings this service started with. Changing startup configuration requires an administrator to update the service configuration and restart it. This screen does not pretend to apply those changes live.</p>
    {show('signin') ? <><h2>Sign-in</h2><dl className="facts"><dt>Provider</dt><dd>{settings.sign_in.provider_origin}</dd><dt>Session duration</dt><dd>{duration(settings.sign_in.session_seconds)}</dd><dt>HTTPS-only session cookie</dt><dd>{settings.sign_in.secure_cookie ? 'Yes' : 'No'}</dd></dl></> : null}
    {show('directory') ? <><h2>Directory</h2><dl className="facts"><dt>Role records</dt><dd>{configured(settings.directory.roles_configured)}</dd></dl></> : null}
    {show('permissions') ? <><h2>Permissions</h2><dl className="facts"><dt>Model version</dt><dd>{settings.permissions.model_version}</dd><dt>Permission projection</dt><dd>{settings.permissions.projection === 'spicedb' ? 'SpiceDB' : 'Local'}</dd></dl></> : null}
    {show('secrets') ? <><h2>Secrets</h2><p>Broker: {configured(settings.secrets.configured)}.</p><p>Credential values are never included in these settings.</p></> : null}
    {show('runtimes') ? <><h2>Machines and profiles</h2><dl className="facts"><dt>Machine records</dt><dd>{configured(settings.runtimes.machines_configured)}</dd><dt>Provisioning records</dt><dd>{configured(settings.runtimes.provisioning_configured)}</dd></dl><p>Configured records do not prove that a runtime is connected or applying profiles.</p></> : null}
    {show('models') && settings.organisation ? <ModelWindows key={settings.organisation.version} organisation={settings.organisation} undeclared={settings.models_undeclared ?? []} saved={() => setRound((now) => now + 1)} /> : null}
    {show('build') ? <><h2>Build</h2><dl className="facts"><dt>Running build</dt><dd>{authority.status === 'ok' ? <code>{authority.data.build}</code>
      : authority.status === 'loading' ? 'Reading…' : authority.refused.refusal.refusal + ': ' + authority.refused.refusal.reason}</dd></dl></> : null}
    {show('storage') ? <><h2>Storage and keys</h2><dl className="facts"><dt>Directory records</dt><dd>{settings.storage.directory_format}</dd><dt>Grant records</dt><dd>{settings.storage.grant_format}</dd><dt>Access request storage</dt><dd>{configured(settings.storage.requests_configured)}</dd></dl><p>Signing keys, credentials and private file paths are not returned to the screen.</p></> : null}
  </>} />;
}
