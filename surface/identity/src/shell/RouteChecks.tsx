import { Navigate, useParams } from "react-router";
import { api, useLoad } from "../api";
import { entries } from "../features/people/directory";
import { Gate } from "../features/signin/Gate";
import { NotFound } from "./NotFound";
import { Access } from "../features/access/Access";
import { Settings } from "../features/settings/Settings";
import { People } from "../features/people/People";
import { IdentityFile } from "../features/file/IdentityFile";
import { MERGED, tabsFor } from "../features/file/tabs";

export function AccessRoute() {
  const { mode = "" } = useParams();
  return ["", "issue", "ask", "can", "reach", "who"].includes(mode) ? <Access /> : <NotFound />;
}
export function SettingsRoute() {
  const { sec = "" } = useParams();
  return ["", "account", "layout", "signin", "directory", "permissions", "secrets", "runtimes", "models", "storage"].includes(
    sec,
  ) ? (
    <Settings />
  ) : (
    <NotFound />
  );
}
export function PeopleRoute() {
  const { view } = useParams();
  return !view || ["all", "people", "agents", "teams", "accounts", "found"].includes(view) ? <People /> : <NotFound />;
}
export function FileRoute() {
  const { id = "", tab } = useParams();
  return !tab || tabsFor(id).some(([key]) => key === tab) || Object.hasOwn(MERGED, tab) ? (
    <IdentityFile />
  ) : (
    <NotFound />
  );
}
export function OneStart() {
  const { id, agent } = useParams();
  const named = id ?? agent;
  return !named ? (
    <Navigate replace to="/dashboard" />
  ) : /^(agent|person|op)-[0-9a-f]{32}$/.test(named) ? (
    <Navigate replace to={"/file/" + encodeURIComponent(named)} />
  ) : (
    <AgentByName name={named} />
  );
}
function AgentByName({ name }: { name: string }) {
  const load = useLoad(api.people, "agent-address:" + name);
  return (
    <Gate
      load={load}
      title="the agent named in this address"
      ok={(view) => {
        const matching = entries(view).filter(
          (entry) => entry.kind === "agent" && entry.state !== "retired" && entry.display_name === name,
        );
        if (matching.length === 1) return <Navigate replace to={"/file/" + encodeURIComponent(matching[0].id)} />;
        return (
          <NotFound next={<a className="btn" href="#/people/view/agents">Open the agents list</a>}>
            {matching.length ? (
              <>More than one agent is called {name}. Choose one from the agents list.</>
            ) : (
              <>No agent called {name} is in the directory you can see.</>
            )}
          </NotFound>
        );
      }}
    />
  );
}
