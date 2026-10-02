"""Nonempty API records made by the old, real install and compared after upgrade."""

import http.client
import json
import secrets

from upgrade_release import limits_body, per_measure_body, zoned


def operation():
    return "op-" + secrets.token_hex(16)


class Browser:
    def __init__(self, port):
        self.port = port
        self.cookie = None

    def ask(self, method, path, body=None, expected_status=200, bearer=None, operator=None):
        connection = http.client.HTTPConnection("127.0.0.1", self.port)
        headers = {"Content-Type": "application/json"}
        if self.cookie:
            headers["Cookie"] = self.cookie
        if bearer is not None:
            headers["Authorization"] = "Bearer " + bearer
        if operator is not None:
            headers["lys-operator"] = operator
        payload = None if body is None else json.dumps(body)
        try:
            connection.request(method, "/api" + path, payload, headers)
            answer = connection.getresponse()
            text = answer.read().decode()
            if answer.status != expected_status:
                raise RuntimeError(f"{method} {path}: HTTP {answer.status}: {text}")
            cookie = answer.getheader("Set-Cookie")
            if cookie:
                self.cookie = cookie.split(";", 1)[0]
            return json.loads(text)
        finally:
            connection.close()


AGENT_LIMIT = {"unit": "tokens", "amount": 100, "period": "day", "zone": "Australia/Melbourne",
               "act": "tell"}


def agent_budget_body(release):
    """The agent's one budget, in the request shape the old release accepts."""
    if release["budgets"] == "limits":
        return limits_body(zoned([AGENT_LIMIT], release["zones"]), 0)
    if release["budgets"] == "per_measure":
        return per_measure_body(AGENT_LIMIT, 0)
    raise RuntimeError(f"no agent budget request for the {release['budgets']} budget model")


def populate(browser, root, release):
    code = (root / "setup-code").read_text().strip()
    password = "Upgrade-" + secrets.token_hex(24) + "-1aA!"
    email = "upgrade-fixture@example.test"
    browser.ask(
        "POST",
        "/setup/administrator",
        {
            "code": code,
            "operation": operation(),
            "display_name": "Upgrade Owner",
            "email": email,
            "password": password,
        },
    )
    browser.ask("POST", "/sign-in", {"email": email, "password": password})
    signed_in = browser.ask("GET", "/me")["person"]
    if signed_in["state"] != "active":
        raise RuntimeError("old setup fixture must exercise its Active administrator")
    owner = signed_in["id"]
    person = browser.ask(
        "POST",
        "/people",
        {
            "operation": operation(),
            "display_name": "Preserved Person",
        },
    )["person"]
    browser.ask(
        "POST",
        f"/identities/{person}/transitions",
        {
            "operation": operation(),
            "transition": "activate",
            "reason": "Upgrade grant holder",
        },
    )
    if browser.ask("GET", f"/identities/{person}")["state"] != "active":
        raise RuntimeError("old fixture grant holder was not activated by its transition")
    registered = browser.ask(
        "POST",
        "/agents",
        {
            "operation": operation(),
            "display_name": "Preserved Agent",
        },
    )
    agent = registered["agent"]
    if registered["responsible"] != owner:
        raise RuntimeError("old fixture registration has the wrong responsible person")
    app = "upgrade_fixture"
    kind = app + ".document"
    browser.ask(
        "POST",
        "/apps",
        {
            "operation": operation(),
            "id": app,
            "name": "Upgrade Fixture",
            "redirects": ["https://upgrade.example.test/callback"],
            "schema": {
                "kinds": {
                    kind: {
                        "actions": ["read"],
                        "relations": {"reader": ["read"]},
                        "parents": [],
                    }
                }
            },
        },
    )
    browser.ask("POST", f"/apps/{app}/approve", {"operation": operation()})
    browser.ask(
        "POST",
        "/service-accounts",
        {
            "operation": operation(),
            "name": "Preserved service account",
            "description": "Old-install upgrade fixture",
        },
    )
    grant = browser.ask(
        "POST",
        "/grants/roots",
        {
            "operation": operation(),
            "route": "api",
            "holder": person,
            "resource": {"kind": kind, "id": "preserved"},
            "relation": "reader",
            "pass_on": {"kind": "use_only"},
            "window": {"starts_at": 0, "ends_at": None},
        },
    )["grant"]
    browser.ask("PUT", f"/budgets/agent/{agent}", agent_budget_body(release))
    browser.ask(
        "POST",
        f"/agents/{agent}/goals",
        {
            "operation": operation(),
            "kind": "goal",
            "words": "Preserve this goal",
            "deadline": 4102444800,
            "reminders": [],
        },
    )
    # A release may record a policy for the agent before this first change;
    # the change is sent on the version read, as the route asks.
    held = browser.ask("GET", f"/agents/{agent}/policy")["policy"]
    browser.ask(
        "POST",
        f"/agents/{agent}/policy",
        {
            "version": 0 if held is None else held["version"],
            "rules": [
                {
                    "id": "preserve-denial",
                    "tool": "Write",
                    "kind": "path_prefix",
                    "target": "/upgrade-fixture/denied",
                    "authority": "hard",
                }
            ],
        },
    )
    browser.ask(
        "POST",
        f"/agents/{agent}/provisioning",
        {
            "operation": operation(),
            "from_version": 0,
            "model_access": ["fixture-model"],
            "tools": ["Read"],
            "skills": [],
            "mcp_servers": [{"name": "fixture-records", "url": "https://mcp.example.test/records"}],
            "instructions": "Preserve this ordinary legacy profile.",
            "note": "Old-install proof",
        },
    )
    return {"owner": owner, "person": person, "agent": agent, "app": app, "grant": grant}


def observe(browser, ids):
    paths = {
        "me": "/me",
        "people": "/directory/people",
        "person": f"/identities/{ids['person']}",
        "agent": f"/agents/{ids['agent']}",
        "grant": f"/grants/{ids['grant']}",
        "app": f"/apps/{ids['app']}",
        "sessions": "/sessions",
        "service_accounts": "/service-accounts",
        "configuration": "/configuration",
        "budgets": f"/budgets/agent/{ids['agent']}",
        "goals": f"/agents/{ids['agent']}/goals",
        "policy": f"/agents/{ids['agent']}/policy",
        "provisioning": f"/agents/{ids['agent']}/provisioning",
    }
    result = {name: browser.ask("GET", path) for name, path in paths.items()}
    budgets = "budgets" if "budgets" in result["budgets"] else "limits"
    for domain, member in [("sessions", "sessions"), ("budgets", budgets), ("goals", "goals")]:
        if not result[domain][member]:
            raise RuntimeError(f"empty {domain} fixture cannot prove an upgrade")
    if not result["policy"]["policy"]["rules"]:
        raise RuntimeError("empty policy cannot prove an upgrade")
    if result["app"]["state"] != "approved":
        raise RuntimeError("fixture app was not approved on the old install")
    if not result["provisioning"]["profile"]["mcp_servers"]:
        raise RuntimeError("empty profile cannot prove legacy MCP configuration survives")
    return result


def migrated_agent(before, after, person):
    value = dict(after)
    expected = {
        "reports_to": {
            "id": person["id"],
            "kind": "person",
            "display_name": person["display_name"],
        },
        "accountable": {"id": person["id"], "display_name": person["display_name"]},
        "gap": None,
    }
    for field, wanted in expected.items():
        if field not in before:
            if field not in value or value[field] != wanted:
                raise RuntimeError(f"upgrade did not materialise the agent's {field}")
            del value[field]
    return value


def migrated_people(before, after):
    if not isinstance(after.get("people"), list):
        raise RuntimeError("upgrade changed the people readback shape")
    value = dict(after)
    old_people = {person["id"]: person for person in before["people"]}
    people = []
    for person in after["people"]:
        old = old_people.get(person["id"])
        if old is None:
            raise RuntimeError("upgrade introduced a person into the directory")
        old_agents = {agent["id"]: agent for agent in old["agents"]}
        agents = []
        for agent in person["agents"]:
            previous = old_agents.get(agent["id"])
            if previous is None:
                raise RuntimeError("upgrade moved or introduced an agent")
            agents.append(migrated_agent(previous, agent, old))
        people.append(dict(person, agents=agents))
    value["people"] = people
    return value


SHIPPED_MODEL_VERSION = 2
LIVE_UNITS = ("dollars", "plan_percent", "context_percent")


def legacy_limit(budget):
    """One per-measure budget as the candidate's limit collection holds it."""
    period = budget["period"]
    limit = {"unit": budget["measure"], "amount": budget["limit"],
             "period": None if period is None else period["length"], "act": budget["act"]}
    if period is not None:
        limit["zone"] = period["zone"]
    return limit


def measured(used, unavailable, effective, name):
    """Live figures: one per effective limit, each a figure or a named gap, never both."""
    if not isinstance(used, list) or len(used) != len(effective):
        raise RuntimeError(f"upgrade did not measure each effective {name} limit")
    for figure, limit in zip(used, effective):
        if set(figure) != {"unit", "period", "figure", "since_ms", "unavailable"} \
                or (figure["unit"], figure["period"]) != (limit["unit"], limit["period"]):
            raise RuntimeError(f"upgrade measured another {name} limit than the one held")
        gap = figure["unavailable"]
        if (figure["figure"] is None) == (gap is None) or (gap is not None and not isinstance(gap, str)) or gap == "":
            raise RuntimeError(f"upgrade {name} figure is neither measured nor named unavailable")
    context = next((figure["unavailable"] for figure in used
                    if figure["unit"] == "context_percent" and figure["unavailable"]), None)
    units = [entry.get("unit") for entry in unavailable] if isinstance(unavailable, list) else None
    if units is None or len(set(units)) != len(units) or not set(units) <= set(LIVE_UNITS) \
            or any(set(entry) != {"unit", "reason"} or not isinstance(entry["reason"], str)
                   or not entry["reason"] for entry in unavailable):
        raise RuntimeError(f"upgrade {name} unavailable units are not each named once")
    named = next((entry["reason"] for entry in unavailable if entry["unit"] == "context_percent"), None)
    if named != context:
        raise RuntimeError(f"upgrade {name} context gap differs from its measured figure")


def kept_limits(before, after, name, pending):
    """A release that already answered limit collections: nothing to map, nothing to hold.

    Such a release refuses a person's own budget edit, so no legacy self-set
    budget exists to be held unconfirmed; its limits must read back unchanged.
    """
    if not isinstance(before.get("limits"), list) or not isinstance(before.get("unconfirmed"), list):
        raise RuntimeError(f"old {name} readback has an unknown shape")
    if pending:
        raise RuntimeError(f"old {name} came from a limits release, which holds nothing for confirmation")
    if before["unconfirmed"]:
        raise RuntimeError(f"old {name} readback already held budgets unconfirmed")
    if after.get("limits") != before["limits"]:
        raise RuntimeError(f"upgrade changed the {name} limits")
    if after.get("unconfirmed") != []:
        raise RuntimeError(f"upgrade held {name} unconfirmed that the old release had confirmed")
    if "effective_limits" in after:
        raise RuntimeError(f"upgrade enforced other {name} than the limits it holds")
    if after != before:
        raise RuntimeError(f"upgrade changed the {name} readback")


def migrated_budgets(before, after, zone, pending):
    """Every old per-measure budget read back in the candidate's limit collection.

    `pending` maps each measure the upgrade must hold for confirmation to the
    full budget that stays enforced; every other measure must be confirmed.
    """
    name = f"{before['holder']['kind']} budgets"
    if "budgets" in after:
        if after != before:
            raise RuntimeError(f"upgrade changed the {name} readback")
        return
    if "budgets" not in before:
        kept_limits(before, after, name, pending)
        return
    if set(before) != {"holder", "budgets"}:
        raise RuntimeError(f"old {name} readback has an unknown shape")
    rows = before["budgets"]
    measures = [row["measure"] for row in rows]
    if len(set(measures)) != len(measures) or not set(pending) <= set(measures):
        raise RuntimeError(f"old {name} readback does not name each measure once")
    limits = [legacy_limit(row) for row in rows]
    latest = max((row["at"] for row in rows), default=None)
    expected = {
        "holder": before["holder"], "limits": limits, "warn_at": None, "zone": zone,
        "version": sum(row["version"] for row in rows),
        "at": latest, "within": [],
    }
    keys = set(expected) | {"by", "used", "unavailable", "unconfirmed"}
    effective = [legacy_limit(pending.get(row["measure"], row)) for row in rows]
    if pending:
        keys.add("effective_limits")
    if set(after) != keys:
        raise RuntimeError(f"upgrade {name} readback members differ: {sorted(set(after) ^ keys)}")
    for field, wanted in expected.items():
        if after[field] != wanted:
            raise RuntimeError(f"upgrade changed the {name} {field}")
    authors = {row["by"] for row in rows if row["at"] == latest}
    if (after["by"] is None) != (latest is None) or (rows and after["by"] not in authors):
        raise RuntimeError(f"upgrade changed who last set the {name}")
    if pending and after["effective_limits"] != effective:
        raise RuntimeError(f"upgrade loosened the {name} effective limits")
    held = after["unconfirmed"]
    if not isinstance(held, list) or len(held) != len(pending):
        raise RuntimeError(f"upgrade did not hold exactly {len(pending)} {name} for confirmation")
    requested = {row["measure"]: row for row in rows}
    seen = set()
    for entry in held:
        if not isinstance(entry, dict) or set(entry) != {"requested", "effective", "reason"}:
            raise RuntimeError(f"upgrade {name} confirmation has an unknown shape")
        measure = entry["requested"].get("measure")
        if measure in seen or measure not in pending:
            raise RuntimeError(f"upgrade held an unexpected {name} measure {measure}")
        seen.add(measure)
        if entry["requested"] != requested[measure]:
            raise RuntimeError(f"upgrade changed the requested {name} {measure}")
        if entry["effective"] != pending[measure]:
            raise RuntimeError(f"upgrade did not keep the full effective {name} {measure}")
        if not isinstance(entry["reason"], str) or not entry["reason"]:
            raise RuntimeError(f"upgrade {name} {measure} confirmation names no reason")
    measured(after["used"], after["unavailable"], effective, name)


def migrated_configuration(before, after, started):
    """The old settings unchanged, the grant model raised only to the shipped one,
    and an organisation zone recorded once by the host setup after the old session began."""
    value = json.loads(json.dumps(after))
    old = before["permissions"]["model_version"]
    wanted = old + 1 if old < SHIPPED_MODEL_VERSION else old
    if value.get("permissions", {}).get("model_version") != wanted:
        raise RuntimeError(f"upgrade configuration model version is not {wanted}")
    value["permissions"]["model_version"] = old
    if "organisation" not in before and "organisation" in value:
        zone = value.pop("organisation")
        if not isinstance(zone, dict) or set(zone) != {"zone", "version", "by", "at"} \
                or not isinstance(zone["zone"], str) or not zone["zone"] \
                or zone["version"] != 1 or zone["by"] != "host_setup" \
                or type(zone["at"]) is not int or zone["at"] < started:
            raise RuntimeError("upgrade configuration organisation is not the first host setting")
    if value != before:
        raise RuntimeError("upgrade changed the configuration readback")


def migrated_goals(before, after):
    """Each old goal unchanged; a goal never deactivated reads back active."""
    if not isinstance(after.get("goals"), list) or len(after["goals"]) != len(before["goals"]):
        raise RuntimeError("upgrade changed the goals readback")
    items = []
    for old, item in zip(before["goals"], after["goals"]):
        goal = dict(item.get("goal") or {})
        if "active" not in old["goal"] and "active" in goal:
            if goal.pop("active") is not True:
                raise RuntimeError("upgrade deactivated one of the goals")
        items.append(dict(item, goal=goal))
    if dict(after, goals=items) != before:
        raise RuntimeError("upgrade changed the goals readback")


PROFILE_DEFAULTS = (("skill_pins", []), ("harness", None), ("permissions", None),
                    ("instructions_mode", "append"), ("session", None))


def migrated_provisioning(before, after):
    """The old profile unchanged, with only the candidate's declared defaults added."""
    value = after
    if isinstance(after.get("profile"), dict) and isinstance(before.get("profile"), dict):
        profile = dict(after["profile"])
        for field, empty in PROFILE_DEFAULTS:
            if field not in before["profile"] and field in profile:
                if profile[field] != empty:
                    raise RuntimeError(f"upgrade changed the provisioning {field} readback")
                del profile[field]
        value = dict(after, profile=profile)
    if value != before:
        raise RuntimeError("upgrade changed the provisioning readback")


def same_records(before, after):
    if before.keys() != after.keys():
        raise RuntimeError("upgrade readback domains differ")
    if not before["sessions"]["sessions"]:
        raise RuntimeError("the old install had no session to preserve")
    started = max(session["started_at"] for session in before["sessions"]["sessions"])
    zone = after.get("configuration", {}).get("organisation", {}).get("zone")
    for name in sorted(before, key=lambda domain: domain != "configuration"):
        value = after[name]
        if name == "agent":
            value = migrated_agent(before[name], value, before[name]["person"])
        if name == "people":
            value = migrated_people(before[name], value)
        if name == "budgets":
            migrated_budgets(before[name], value, zone, {})
            continue
        if name == "configuration":
            migrated_configuration(before[name], value, started)
            continue
        if name == "goals":
            migrated_goals(before[name], value)
            continue
        if name == "provisioning":
            migrated_provisioning(before[name], value)
            continue
        if before[name] != value:
            raise RuntimeError(f"upgrade changed the {name} readback")


def restored_records(before, after):
    """After a put-back the previous build runs on the data it last ran on: every
    domain reads back exactly as it did before the upgrade, with nothing migrated."""
    if before.keys() != after.keys():
        raise RuntimeError("put-back readback domains differ")
    for name in sorted(before):
        if before[name] != after[name]:
            raise RuntimeError(f"put-back changed the {name} readback")


def admitted_after_upgrade(browser, administrator):
    """The original Active administrator's session can still make a real change."""
    if administrator["person"]["state"] != "active":
        raise RuntimeError("old administrator must be active")
    if browser.ask("GET", "/me") != administrator:
        raise RuntimeError("upgrade changed the original administrator or login")
    person = browser.ask(
        "POST",
        "/people",
        {
            "operation": operation(),
            "display_name": "Created After Upgrade",
        },
    )["person"]
    kept = browser.ask("GET", f"/identities/{person}")
    if kept["id"] != person or kept["display_name"] != "Created After Upgrade":
        raise RuntimeError("old Active administrator cannot write and read after upgrade")
    if browser.ask("GET", "/me") != administrator:
        raise RuntimeError("write changed the original administrator or login")
    return person
