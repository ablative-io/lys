import { EstateApproval } from './features/import/EstateApproval';
import { ServiceAccounts } from './features/service-accounts/ServiceAccounts';
import { Roles } from './features/roles/Roles';
import { Network } from './features/network/Network';
import { Connections } from './features/connections/Connections';
import { SecretsPage } from './features/secrets/SecretsPage';
import { Sessions } from './features/sessions/Sessions';
import { Reviews } from './features/reviews/Reviews';
import { Requests } from './features/requests/Requests';
import { Navigate, Route, Routes, useParams } from 'react-router';
import { Access } from './features/access/Access';
import { IdentityFile } from './features/file/IdentityFile';
import { You } from './features/me/You';
import { Settings } from './features/settings/Settings';
import { People } from './features/people/People';
import { AddAgent } from './features/people/AddAgent';
import { AddPerson } from './features/people/AddPerson';
import { IssueRoot } from './features/grants/IssueRoot';
import { Model } from './features/access/Model';
import { Graph } from './features/access/Graph';
import { Resources } from './features/access/Resources';
import { Apps } from './features/apps/Apps';
import { SessionCanvas } from './features/runtime/SessionCanvas';

/** Every screen and tab has its own address (conformance 9.1). */
export function AppRoutes() {
  return (
    <Routes>
      <Route path="/access/import" element={<EstateApproval />} />
      <Route path="/roles/:id?" element={<Roles />} />
      <Route path="/network" element={<Network />} />
      <Route path="/service-accounts" element={<ServiceAccounts />} />
      <Route path="/connections" element={<Connections />} />
      <Route path="/access/issue" element={<IssueRoot />} />
      <Route path="/model" element={<Model />} />
      <Route path="/apps" element={<Apps />} />
      <Route path="/graph/:id?" element={<Graph />} />
      <Route path="/resources" element={<Resources />} />
      <Route path="/secrets/:section?" element={<SecretsPage />} />
      <Route path="/vault" element={<SecretsPage />} />
      <Route path="/sessions" element={<Sessions />} />
      <Route path="/runtime/:session?" element={<Navigate replace to="/canvas" />} />
      <Route path="/usage/:agent?" element={<UsageMoved />} />
      <Route path="/canvas/:agent?" element={<SessionCanvas />} />
      <Route path="/team/:agent?" element={<OneStart />} />
      <Route path="/people/:agent" element={<People />} />
      <Route path="/reviews" element={<Reviews />} />
      <Route path="/requests" element={<Requests />} />
      <Route path="/agents/new" element={<AddAgent />} />
      <Route path="/people/new" element={<AddPerson />} />
      <Route path="/people" element={<People />} />
      <Route path="/file/:id/start" element={<OneStart />} />
      <Route path="/file/:id/:tab?" element={<IdentityFile />} />
      <Route path="/me" element={<You />} />
      <Route path="/account/:id" element={<AccountMoved />} />
      <Route path="/settings/:sec?" element={<Settings />} />
      <Route path="/access/:mode?/:arg?" element={<Access />} />
      <Route path="*" element={<You />} />
    </Routes>
  );
}

/** An agent's budgets and goals are its own settings, on its file. */
function UsageMoved() {
  const { agent } = useParams();
  return <Navigate replace to={agent ? '/file/' + encodeURIComponent(agent) + '/budgets' : '/people'} />;
}

/** A person's Lys account is on the Credentials tab of their own page. */
function AccountMoved() {
  const { id = '' } = useParams();
  return <Navigate replace to={'/file/' + encodeURIComponent(id) + '/credentials'} />;
}

/** An agent has one start: on the Overview of its own page. Older addresses for it arrive there. */
function OneStart() {
  const { id, agent } = useParams();
  const named = id ?? agent;
  return <Navigate replace to={named ? '/file/' + encodeURIComponent(named) : '/me'} />;
}
