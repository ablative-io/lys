"""Nonempty API records made by the old, real install and compared after upgrade."""

import http.client
import json
import secrets


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


def populate(browser, root):
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
    browser.ask(
        "PUT",
        f"/budgets/agent/{agent}",
        {
            "version": 0,
            "measure": "tokens",
            "limit": 100,
            "period": {"length": "day", "zone": "Australia/Melbourne"},
            "act": "tell",
        },
    )
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
    browser.ask(
        "POST",
        f"/agents/{agent}/policy",
        {
            "version": 0,
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
    for domain, member in [("sessions", "sessions"), ("budgets", "budgets"), ("goals", "goals")]:
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


def same_records(before, after):
    if before.keys() != after.keys():
        raise RuntimeError("upgrade readback domains differ")
    for name in before:
        value = after[name]
        if name == "agent":
            value = migrated_agent(before[name], value, before[name]["person"])
        if name == "people":
            value = migrated_people(before[name], value)
        # This fixture's agent budget cannot need personal-budget confirmation.
        # Accept only the declared additive empty field, never erase its contents.
        if name == "budgets" and "unconfirmed" not in before[name] and "unconfirmed" in value:
            if value.get("unconfirmed") != []:
                raise RuntimeError("upgrade changed the budgets confirmation readback")
            value = {key: entry for key, entry in value.items() if key != "unconfirmed"}
        if name == "provisioning" and isinstance(value.get("profile"), dict):
            profile = dict(value["profile"])
            for field, empty in (("skill_pins", []), ("harness", None), ("permissions", None)):
                if field not in before[name]["profile"] and field in profile:
                    if profile[field] != empty:
                        raise RuntimeError(f"upgrade changed the provisioning {field} readback")
                    del profile[field]
            value = dict(value, profile=profile)
        if before[name] != value:
            raise RuntimeError(f"upgrade changed the {name} readback")
    if not before["sessions"]["sessions"]:
        raise RuntimeError("the old install had no session to preserve")


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
