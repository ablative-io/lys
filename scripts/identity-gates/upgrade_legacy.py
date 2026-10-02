"""Legacy authority cases created through the real old installer and HTTP APIs."""

import hashlib
import http.client
import json
from pathlib import Path
import secrets
from urllib.parse import urlsplit

from upgrade_fixture import Browser, migrated_budgets, operation


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
    added = {}
    for who in (person, ids["agent"], ids["owner"]):
        added[who] = member.ask("POST", f"/teams/{team}/members",
                                {"operation": operation(), "member": who})["recorded"]
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
            "added": added, "first": first, "original": original,
            "team_before": admin.ask("GET", f"/teams/{team}"),
            "budgets_before": admin.ask("GET", path)}, member


def expected_hold(team, member, added):
    """The hold the candidate derives from the old adding record, naming who added it."""
    digest = hashlib.sha256(f"lys/teams/legacy-membership/v1\n{added['operation']}".encode())
    login = added["by"]
    return {"operation": "op-" + digest.hexdigest()[:32], "team": team, "member": member,
            "reason": f"membership added by {login['provider']}/{login['subject']} under operation "
                      f"`{added['operation']}` has no current authority for this member",
            "at": added["at"]}


def migrated_team(old, team, fixture):
    """Every old team member unchanged; a legacy team gains no parent or lead, and
    holds exactly its foreign members, in membership order, from their adding records."""
    for key, value in old.items():
        if key not in team or team[key] != value:
            raise RuntimeError(f"legacy team {key} changed during upgrade")
    added = {key: team[key] for key in team.keys() - old.keys()}
    unknown = sorted(set(added) - {"parent", "lead", "held"})
    if unknown:
        raise RuntimeError(f"legacy team gained unknown members {unknown}")
    for key in ("parent", "lead"):
        if key in added and added[key] is not None:
            raise RuntimeError(f"legacy team gained a {key} during upgrade")
    held = team.get("held")
    if not isinstance(held, list) or [entry.get("member") for entry in held] != [
            who for who in old["members"] if who in fixture["foreign"]] or len(held) != 2:
        raise RuntimeError("legacy team did not retain exactly its two foreign memberships as held")
    if any(not isinstance(entry.get("reason"), str) or not entry["reason"] for entry in held):
        raise RuntimeError("legacy team hold lacks its named reason")
    if "held" in added and held != [expected_hold(old["id"], who, fixture["added"][who])
                                    for who in old["members"] if who in fixture["foreign"]]:
        raise RuntimeError("legacy team hold does not name its old adding record")
    return held


def verify(admin, fixture):
    """Require exact historical values, two named holds, and three effective budgets."""
    team = admin.ask("GET", f"/teams/{fixture['team']}")
    held = migrated_team(fixture["team_before"], team, fixture)
    budgets = admin.ask("GET", f"/budgets/person/{fixture['person']}")
    zone = admin.ask("GET", "/configuration")["organisation"]["zone"]
    effective = {"context_percent": fixture["first"], **fixture["original"]}
    if {row["measure"] for row in fixture["budgets_before"].get("budgets", [])} != set(effective):
        raise RuntimeError("legacy pending budget measures differ")
    migrated_budgets(fixture["budgets_before"], budgets, zone, effective)
    return {"held_members": len(held), "unconfirmed_budgets": len(budgets["unconfirmed"])}
