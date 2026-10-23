import { EstateApproval } from './features/import/EstateApproval';
import { Roles } from './features/roles/Roles';
import { Network } from './features/network/Network';
import { Connections } from './features/connections/Connections';
import { SecretsPage } from './features/secrets/SecretsPage';
import { Reviews } from './features/reviews/Reviews';
import { Requests } from './features/requests/Requests';
import { PairPage } from './features/pair/PairPage';
import { Navigate, Route, Routes, useParams } from 'react-router';
import { You } from './features/me/You';
import { Dashboard } from './features/dashboard/Dashboard';
import { Drafts } from './features/drafts/Drafts';
import { People } from './features/people/People';
import { AddAgent } from './features/people/AddAgent';
import { AddPerson } from './features/people/AddPerson';
import { Model } from './features/access/Model';
import { Graph } from './features/access/Graph';
import { Resources } from './features/access/Resources';
import { Apps } from './features/apps/Apps';
import { ConnectApp } from './features/apps/ConnectApp';
import { SessionCanvas } from './features/runtime/SessionCanvas';
import { ServiceAccountsList } from './features/service-accounts/ServiceAccounts';
import { ConfigTabs } from './features/settings/ConfigTabs';
import { PageHeader } from './shell/PageHeader';
import { NotFound } from './shell/NotFound';
import { AccessRoute, FileRoute, OneStart, PeopleRoute, SettingsRoute } from './shell/RouteChecks';

/** Every screen and tab has its own address (conformance 9.1). */
export function AppRoutes() {
  return (
    <Routes>
      <Route path="/access/import" element={<EstateApproval />} />
      <Route path="/access/drafts" element={<Drafts />} />
      <Route path="/roles/:id?" element={<Roles />} />
      <Route path="/network" element={<Network />} />
      <Route path="/service-accounts" element={<Navigate replace to="/settings/service-accounts" />} />
      <Route path="/connections" element={<Connections />} />
      <Route path="/model" element={<Model />} />
      <Route path="/apps" element={<Apps />} />
      <Route path="/apps/connect" element={<ConnectApp />} />
      <Route path="/graph/:id?" element={<Graph />} />
      <Route path="/resources" element={<Resources />} />
      <Route path="/secrets/:section?" element={<SecretsPage />} />
      <Route path="/vault" element={<Navigate replace to="/secrets" />} />
      <Route path="/sessions" element={<Navigate replace to="/me" />} />
      <Route path="/runtime/:session?" element={<Navigate replace to="/canvas" />} />
      <Route path="/usage/:agent?" element={<UsageMoved />} />
      <Route path="/canvas/:agent?" element={<SessionCanvas />} />
      <Route path="/team/:agent?" element={<OneStart />} />
      <Route path="/people/view/:view" element={<PeopleRoute />} />
      <Route path="/people/:agent" element={<People />} />
      <Route path="/reviews" element={<Reviews />} />
      <Route path="/requests/:request?" element={<Requests />} />
      <Route path="/pair" element={<PairPage />} />
      <Route path="/agents/new" element={<AddAgent />} />
      <Route path="/people/new" element={<AddPerson />} />
      <Route path="/people" element={<People />} />
      <Route path="/file/:id/start" element={<OneStart />} />
      <Route path="/file/:id/:tab?" element={<FileRoute />} />
      <Route path="/" element={<Dashboard />} />
      <Route path="/dashboard" element={<Dashboard />} />
      <Route path="/me" element={<You />} />
      <Route path="/account/:id" element={<Navigate replace to="/settings/account" />} />
      <Route path="/settings/service-accounts" element={<ServiceAccountsPage />} />
      <Route path="/settings/:sec?" element={<SettingsRoute />} />
      <Route path="/access/:mode?/:arg?" element={<AccessRoute />} />
      <Route path="*" element={<NotFound />} />
    </Routes>
  );
}

/** Service accounts are kept under Settings, beside the person's own account and the service's settings (domain 02's ask). */
function ServiceAccountsPage() {
  return (
    <div className="page fill">
      <PageHeader title="Settings" subtitle="The accounts your agents use for work, kept so everyone knows what each is for." />
      <ConfigTabs on="accounts" />
      <ServiceAccountsList />
    </div>
  );
}

/** An agent's budgets and goals are its own settings, on its file. */
function UsageMoved() {
  const { agent } = useParams();
  return <Navigate replace to={agent ? '/file/' + encodeURIComponent(agent) + '/budgets' : '/people'} />;
}

/** An agent has one start: on the Overview of its own page. Older addresses for it arrive there. */
