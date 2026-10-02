"""What an old release accepts and answers, read from that release's own source tree.

The proof never keeps a list of commits: the request struct a route deserialises,
and the answer struct it serialises, say which shape the release speaks.
"""

import re

BUDGETS_API = "crates/lys-identity-server/src/budgets_api.rs"
TEAMS_API = "crates/lys-identity-server/src/teams_api.rs"
INTENT = "crates/lys/src/identity/upgrade/intent.rs"
REFUSALS = "crates/lys-identity-server/src/openapi_refusals.rs"

# `PUT /budgets/{kind}/{id}`: one measure per request, or the holder's whole collection.
PER_MEASURE_BODY = frozenset({"version", "measure", "limit", "period", "act"})
LIMITS_BODY = frozenset({"version", "limits", "warn_at"})


def struct_fields(text, name, path):
    """The field names of `struct name { ... }` in Rust source, in order."""
    match = re.search(r"\bstruct " + re.escape(name) + r"\s*\{(.*?)\n\}", text, re.S)
    if match is None:
        raise RuntimeError(f"{path} declares no struct {name}")
    body = re.sub(r"//[^\n]*", "", match.group(1))
    body = re.sub(r"#\[[^\]]*\]", "", body)
    fields = re.findall(r"^\s*(?:pub(?:\([^)]*\))?\s+)?([a-z_][a-z0-9_]*)\s*:(?!:)", body, re.M)
    if not fields:
        raise RuntimeError(f"{path} struct {name} names no fields")
    return fields


def budget_model(text, path=BUDGETS_API):
    """`per_measure` or `limits`, from the release's own `BudgetBody`."""
    fields = frozenset(struct_fields(text, "BudgetBody", path))
    if fields == PER_MEASURE_BODY:
        return "per_measure"
    if fields == LIMITS_BODY:
        return "limits"
    raise RuntimeError(f"{path} BudgetBody has fields {sorted(fields)}, which the proof cannot seed")


def team_model(text, path=TEAMS_API):
    """`guarded` when the release's own team answer names held memberships, else `unguarded`.

    A release whose `TeamView` carries `held` derives and keeps its own holds,
    so no membership without current authority survives into the candidate.
    """
    fields = struct_fields(text, "TeamView", path)
    return "guarded" if "held" in fields else "unguarded"


def put_back_model(text, path=INTENT):
    """`keeps_data` when the release's upgrade record knows the kept-data step, else `no_data`.

    A `no_data` release's installer puts back binaries and files but never the
    data the candidate wrote, so once the candidate has started it cannot bring
    its own build back; that install recovers through the candidate's installer.
    """
    match = re.search(r"\benum Step\s*\{(.*?)\n\}", text, re.S)
    if match is None:
        raise RuntimeError(f"{path} declares no enum Step")
    return "keeps_data" if re.search(r"^\s*DataKept\b", match.group(1), re.M) else "no_data"


def zone_model(text):
    """`organisation` when the release's published refusals name BudgetZoneRefused, else `explicit`.

    Such a release counts every new limit in the organisation zone and keeps an
    explicit zone only on a limit migrated with one.
    """
    return "organisation" if "BudgetZoneRefused" in text else "explicit"


def zoned(limits, zones):
    """`limits` as a release with the `zones` model accepts them as new."""
    if zones == "explicit":
        return [dict(limit) for limit in limits]
    if zones != "organisation":
        raise RuntimeError(f"no limits for a release whose zones are {zones!r}")
    return [{key: value for key, value in limit.items() if key != "zone"} for limit in limits]


def old_release(read):
    """The old release's request and answer shapes; `read` returns a source file's text."""
    return {
        "budgets": budget_model(read(BUDGETS_API)),
        "teams": team_model(read(TEAMS_API)),
        "put_back": put_back_model(read(INTENT)),
        "zones": zone_model(read(REFUSALS)),
    }


def limits_body(limits, version):
    """The request a `limits` release accepts: the holder's whole collection."""
    return {"version": version, "limits": [dict(limit) for limit in limits], "warn_at": None}


def per_measure_body(limit, version):
    """The request a `per_measure` release accepts for one limit of the same meaning."""
    period = limit["period"]
    if (period is None) != ("zone" not in limit):
        raise RuntimeError(f"a per-measure {limit['unit']} limit names a zone exactly when it has a period")
    return {
        "version": version,
        "measure": limit["unit"],
        "limit": limit["amount"],
        "period": None if period is None else {"length": period, "zone": limit["zone"]},
        "act": limit["act"],
    }
