"""Legacy authority cases created through the real old installer and HTTP APIs."""

import http.client
import json
from pathlib import Path
import secrets
from urllib.parse import urlsplit

from upgrade_fixture import Browser, operation


def issuer_person(root, config, email):
    """Create a disposable issuer account; never print its key or response body."""
    settings = config["sign_in_providers"]
    endpoint = urlsplit(settings["api"])
    if endpoint.scheme != "http" or endpoint.hostname not in ("localhost", "127.0.0.1", "::1"):
        raise RuntimeError("old fixture issuer must be on loopback")
    key_path = Path(settings["api_key_file"]).resolve()
    if not key_path.is_relative_to(root):
        raise RuntimeError("old fixture issuer key leaves its disposable root")
    connection = http.client.HTTPConnection(endpoint.hostname, endpoint.port)
    try:
        connection.request("POST", endpoint.path.rstrip("/") + "/users", json.dumps({
            "email": email, "given_name": "Legacy Person", "family_name": None,
            "language": "en", "groups": None, "roles": [], "user_expires": None,
        }), {"Content-Type": "application/json",
             "Authorization": "API-Key " + key_path.read_text().strip()})
        answer = connection.getresponse()
        body = answer.read()
        if answer.status not in (200, 201):
            raise RuntimeError(f"fixture issuer create refused HTTP {answer.status}")
        return json.loads(body)["id"]
    finally:
        connection.close()


def seed(admin, root, ids):
    """Exercise the old self-budget and unguarded team-member routes as a real user."""
    config = json.loads((root / "identity.json").read_text())
    email = "legacy-member@example.test"
    password = "Legacy-" + secrets.token_hex(24) + "-1aA!"
    subject = issuer_person(root, config, email)
    person = ids["person"]
    admin.ask("POST", f"/people/{person}/logins", {
        "operation": operation(), "issuer": config["issuer"], "subject": subject,
    })
    admin.ask("POST", f"/directory/people/{person}/account/password", {"password": password})
    member = Browser(admin.port)
    member.ask("POST", "/sign-in", {"email": email, "password": password})
    if member.ask("GET", "/me")["person"]["id"] != person:
        raise RuntimeError("old legacy fixture did not sign in as its ordinary person")
    team = member.ask("POST", "/teams", {
        "operation": operation(), "name": "Legacy team", "description": "Upgrade authority proof",
    })["id"]
    for who in (person, ids["agent"], ids["owner"]):
        member.ask("POST", f"/teams/{team}/members", {"operation": operation(), "member": who})
    path = f"/budgets/person/{person}"
    first = member.ask("PUT", path, {"version": 0, "measure": "context_percent",
        "limit": 60, "period": None, "act": "tell"})
    original = {}
    for measure, limit in (("tokens", 100), ("running_ms", 1000)):
        original[measure] = admin.ask("PUT", path, {"version": 0, "measure": measure,
            "limit": limit, "period": {"length": "week", "zone": "UTC"}, "act": "stop"})
        member.ask("PUT", path, {"version": 1, "measure": measure,
            "limit": 200 if measure == "tokens" else limit,
            "period": {"length": "day" if measure == "running_ms" else "week", "zone": "UTC"},
            "act": "stop"})
    return {"team": team, "person": person, "foreign": [ids["agent"], ids["owner"]],
            "first": first, "original": original,
            "team_before": admin.ask("GET", f"/teams/{team}"),
            "budgets_before": admin.ask("GET", path)}, member


def verify(admin, fixture):
    """Require exact historical values, two named holds, and three effective budgets."""
    team = admin.ask("GET", f"/teams/{fixture['team']}")
    old = fixture["team_before"]
    held = team.get("held")
    if not isinstance(held, list) or {entry["member"] for entry in held} != set(fixture["foreign"]) or len(held) != 2:
        raise RuntimeError("legacy team did not retain exactly its two foreign memberships as held")
    if any(not entry.get("reason") for entry in held):
        raise RuntimeError("legacy team hold lacks its named reason")
    if {key: value for key, value in team.items() if key != "held"} != old:
        raise RuntimeError("legacy team membership or history changed during upgrade")
    budgets = admin.ask("GET", f"/budgets/person/{fixture['person']}")
    if {key: value for key, value in budgets.items() if key != "unconfirmed"} != fixture["budgets_before"]:
        raise RuntimeError("legacy requested budgets changed during upgrade")
    pending = budgets.get("unconfirmed")
    if not isinstance(pending, list) or len(pending) != 3:
        raise RuntimeError("legacy budgets did not name all three pending confirmations")
    effective = {"context_percent": fixture["first"], **fixture["original"]}
    requested = {entry["measure"]: entry for entry in budgets["budgets"]}
    for entry in pending:
        measure = entry["requested"]["measure"]
        if entry["requested"] != requested[measure] or entry["effective"] != effective[measure] or not entry.get("reason"):
            raise RuntimeError(f"legacy {measure} did not preserve its full effective budget")
    if {entry["requested"]["measure"] for entry in pending} != set(effective):
        raise RuntimeError("legacy pending budget measures differ")
    return {"held_members": len(held), "unconfirmed_budgets": len(pending)}
