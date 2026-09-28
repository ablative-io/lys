import { Route, Routes } from 'react-router';
import { Access } from './features/access/Access';
import { IdentityFile } from './features/file/IdentityFile';
import { You } from './features/me/You';
import { NotYet, SCREENS } from './features/notyet/NotYet';
import { Settings } from './features/notyet/Settings';
import { People } from './features/people/People';
import { Manage } from './features/people/Manage';
import { IssueRoot } from './features/grants/IssueRoot';
import { Model } from './features/access/Model';
import { Graph } from './features/access/Graph';
import { Resources } from './features/access/Resources';

/** Every screen and tab has its own address (conformance 9.1). */
export function AppRoutes() {
  return (
    <Routes>
      <Route path="/directory/manage" element={<Manage />} />
      <Route path="/access/issue" element={<IssueRoot />} />
      <Route path="/model" element={<Model />} />
      <Route path="/graph/:id?" element={<Graph />} />
      <Route path="/resources" element={<Resources />} />
      <Route path="/people" element={<People />} />
      <Route path="/file/:id/:tab?" element={<IdentityFile />} />
      <Route path="/me" element={<You />} />
      <Route path="/settings/:sec?" element={<Settings />} />
      <Route path="/access/:mode?/:arg?" element={<Access />} />
      {[...Object.keys(SCREENS).filter((view) => view !== 'model' && view !== 'resources' && view !== 'graph'), 'vault'].map((view) => (
        <Route key={view} path={`/${view}/*`} element={<NotYet />} />
      ))}
      <Route path="*" element={<People />} />
    </Routes>
  );
}
