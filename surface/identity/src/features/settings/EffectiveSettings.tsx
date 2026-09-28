/** Configuration pages show effective server values and link to supported management screens. */
import { request, useLoad } from '../../api';
import { Gate } from '../signin/Gate';

interface Configuration {
  source: 'startup_configuration'; mutable_in_browser: false;
  sign_in: { provider_origin: string; session_seconds: number; secure_cookie: boolean };
  directory: { roles_configured: boolean };
  permissions: { model_version: number; projection: 'local' | 'spicedb' };
  secrets: { configured: boolean };
  runtimes: { machines_configured: boolean; provisioning_configured: boolean };
  storage: { directory_format: string; grant_format: string; requests_configured: boolean };
}
const configured = (value: boolean) => value ? 'Configured' : 'Not configured';

export function EffectiveSettings({ section }: { section: string }) {
  const load = useLoad(() => request<Configuration>('/configuration'), 'effective-configuration');
  return <Gate load={load} title="Configuration" ok={(settings) => <>
    <p className="note">These are the settings this service started with. Changing startup configuration requires an administrator to update the service configuration and restart it. This screen does not pretend to apply those changes live.</p>
    {section === 'signin' ? <><h2>Sign-in</h2><dl className="facts"><dt>Provider</dt><dd>{settings.sign_in.provider_origin}</dd><dt>Session duration</dt><dd>{settings.sign_in.session_seconds} seconds</dd><dt>HTTPS-only session cookie</dt><dd>{settings.sign_in.secure_cookie ? 'Yes' : 'No'}</dd></dl><p><a className="btn" href="#/sessions">Manage signed-in sessions</a> <a className="btn" href="#/directory/manage?action=login">Connect a sign-in account</a></p></> : null}
    {section === 'directory' ? <><h2>Directory</h2><dl className="facts"><dt>Role records</dt><dd>{configured(settings.directory.roles_configured)}</dd></dl><p><a className="btn" href="#/directory/manage">Manage directory</a> <a className="btn" href="#/roles">Roles and assignments</a></p></> : null}
    {section === 'permissions' ? <><h2>Permissions</h2><dl className="facts"><dt>Model version</dt><dd>{settings.permissions.model_version}</dd><dt>Permission projection</dt><dd>{settings.permissions.projection === 'spicedb' ? 'SpiceDB' : 'Local'}</dd></dl><p><a className="btn" href="#/model">Read permission model</a> <a className="btn" href="#/access">Check access</a> <a className="btn" href="#/requests">Access requests</a></p></> : null}
    {section === 'secrets' ? <><h2>Secrets</h2><p>Broker: {configured(settings.secrets.configured)}.</p><p>Credential values are never included in these settings.</p><a className="btn" href="#/secrets">Open secrets</a></> : null}
    {section === 'runtimes' ? <><h2>Machines and profiles</h2><dl className="facts"><dt>Machine records</dt><dd>{configured(settings.runtimes.machines_configured)}</dd><dt>Provisioning records</dt><dd>{configured(settings.runtimes.provisioning_configured)}</dd></dl><p>Configured records do not prove that a runtime is connected or applying profiles.</p><a className="btn" href="#/network">Manage machines</a> <a className="btn" href="#/people">Agent profiles</a></> : null}
    {section === 'storage' ? <><h2>Storage and keys</h2><dl className="facts"><dt>Directory records</dt><dd>{settings.storage.directory_format}</dd><dt>Grant records</dt><dd>{settings.storage.grant_format}</dd><dt>Access request storage</dt><dd>{configured(settings.storage.requests_configured)}</dd></dl><p>Signing keys, credentials and private file paths are not returned to the screen.</p></> : null}
  </>} />;
}
