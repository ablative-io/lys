import { useLocation } from "react-router";
import type { ReactNode } from "react";

export function NotFound({ children }: { children?: ReactNode }) {
  const address = useLocation();
  return (
    <div className="page">
      <div className="head">
        <div>
          <h1>Not found</h1>
          <p className="sub">{children ?? "There is no page at this address."}</p>
        </div>
      </div>
      <p className="note">The address asked for was {address.pathname + address.search}. It may be from an old link.</p>
      <p className="chain">
        <a className="btn" href="#/dashboard">Go to the Dashboard</a>
        <a className="btn" href="#/people/view/agents">Open the agents list</a>
      </p>
    </div>
  );
}
