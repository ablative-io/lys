import { useLocation } from "react-router";
import type { ReactNode } from "react";
import { Act } from "./Act";
import { PageHeader } from "./PageHeader";
import { useShell } from "./ShellContext";

/** An address with no page: what was asked for, and the next step. `next` names a better one where the caller knows it (an old agent link: the agents list); otherwise Search. */
export function NotFound({ children, next }: { children?: ReactNode; next?: ReactNode }) {
  const address = useLocation();
  const shell = useShell();
  return (
    <div className="page">
      <PageHeader title="Not found" subtitle={children ?? "There is no page at this address."} />
      <p className="note">The address asked for was {address.pathname + address.search}. It may be from an old link.</p>
      <p className="chain">
        <a className="btn" href="#/">Go Home</a>
        {next ?? <Act symbol="search" name="Search for the page by its name" word="Search" onClick={shell.openPalette} />}
      </p>
    </div>
  );
}
