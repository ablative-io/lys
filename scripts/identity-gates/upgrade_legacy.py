"""Legacy authority cases created through the real old installer and HTTP APIs."""

import hashlib
import http.client
import json
from pathlib import Path
import secrets
from urllib.parse import urlsplit

from upgrade_fixture import Browser, migrated_budgets, operation
from upgrade_release import limits_body, per_measure_body, zoned


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


# The personal budget every release is given, in the meaning the old self-edit
# scenario starts from: the person's own context level, and two weekly limits
# an administrator set.
PERSON_LIMITS = [
    {"unit": "context_percent", "amount": 60, "period": None, "act": "tell"},
    {"unit": "tokens", "amount": 100, "period": "week", "zone": "UTC", "act": "stop"},
    {"unit": "running_ms", "amount": 1000, "period": "week", "zone": "UTC", "act": "stop"},
]
# The member's own later edit: more tokens, and running time counted daily.
SELF_EDIT = {"tokens": {"amount": 200}, "running_ms": {"period": "day"}}


def edited(limit):
    return dict(limit, **SELF_EDIT.get(limit["unit"], {}))


def refused(browser, method, path, body, status, refusal):
    """An act the old release must refuse by name, writing nothing."""
    answer = browser.ask(method, path, body, expected_status=status)
    if answer.get("refusal") != refusal:
        raise RuntimeError(f"old release refused {path} as {answer.get('refusal')}, not {refusal}")
    return refusal


def seed_team(admin, member, release, ids):
    """A team the member owns with two foreign members, added as the old release allows."""
    person = ids["person"]
    team = member.ask("POST", "/teams", {
        "operation": operation(), "name": "Legacy team", "description": "Upgrade authority proof",
    })["id"]
    added = {person: member.ask("POST", f"/teams/{team}/members",
                                {"operation": operation(), "member": person})["recorded"]}
    path = f"/teams/{team}/members"
    if release["teams"] == "unguarded":
        for who in (ids["agent"], ids["owner"]):
            added[who] = member.ask("POST", path, {"operation": operation(), "member": who})["recorded"]
        return team, added, {}
    if release["teams"] != "guarded":
        raise RuntimeError(f"no team seed for the {release['teams']} team model")
    refusals = {
        "agent": refused(member, "POST", path, {"operation": operation(), "member": ids["agent"]},
                         403, "not_permitted"),
        "person": refused(member, "POST", path, {"operation": operation(), "member": ids["owner"]},
                          403, "NotAdmitted"),
    }
    for who in (ids["agent"], ids["owner"]):
        added[who] = admin.ask("POST", path, {"operation": operation(), "member": who})["recorded"]
    return team, added, refusals


def seed_budgets(admin, member, release, person):
    """The same limits, periods and acts in the shape the old release accepts."""
    path = f"/budgets/person/{person}"
    if release["budgets"] == "per_measure":
        first = member.ask("PUT", path, per_measure_body(PERSON_LIMITS[0], 0))
        original = {}
        for limit in PERSON_LIMITS[1:]:
            original[limit["unit"]] = admin.ask("PUT", path, per_measure_body(limit, 0))
            member.ask("PUT", path, per_measure_body(edited(limit), 1))
        return {"first": first, "original": original}
    if release["budgets"] != "limits":
        raise RuntimeError(f"no personal budget seed for the {release['budgets']} budget model")
    given = zoned(PERSON_LIMITS, release["zones"])
    set_answer = admin.ask("PUT", path, limits_body(given, 0))
    if set_answer["limits"] != given or set_answer["unconfirmed"]:
        raise RuntimeError("old release did not hold the personal limits it was given")
    own_edit = limits_body([edited(limit) for limit in given], set_answer["version"])
    return {"limits": given,
            "refusals": {"self_edit": refused(member, "PUT", path, own_edit, 403, "not_permitted")}}


def seed(admin, root, ids, release):
    """Exercise the old self-budget and team-member routes as a real user.

    A release whose own routes already refuse the member's self-edit and foreign
    adds is asked them anyway, and must refuse each by name: those legacy cases
    cannot exist there, and the proof records that they were refused.
    """
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
    team, added, team_refusals = seed_team(admin, member, release, ids)
    budgets = seed_budgets(admin, member, release, person)
    refusals = dict(team_refusals, **budgets.pop("refusals", {}))
    return {"release": dict(release), "team": team, "person": person,
            "foreign": [ids["agent"], ids["owner"]], "added": added, **budgets,
            "refusals": refusals,
            "team_before": admin.ask("GET", f"/teams/{team}"),
            "budgets_before": admin.ask("GET", f"/budgets/person/{person}")}, member


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


def kept_team(old, team, fixture):
    """A team from a guarded release: every member was admitted by authority the
    release itself checked, so the upgrade holds none and changes nothing."""
    if old.get("held") != [] or old.get("parent") is not None or old.get("lead") is not None:
        raise RuntimeError("guarded release's team was not seeded unheld and unnested")
    if old.get("members") != [fixture["person"], *fixture["foreign"]]:
        raise RuntimeError("guarded release's team does not hold the member and both foreign members")
    for key, value in old.items():
        if key not in team or team[key] != value:
            raise RuntimeError(f"guarded team {key} changed during upgrade")
    if team != old:
        raise RuntimeError(f"guarded team gained members {sorted(team.keys() - old.keys())}")
    return team["held"]


def verify(admin, fixture):
    """Old-model releases: exact historical values, two named holds, and three effective
    budgets. Guarded and limits releases: the same team and limits, nothing held."""
    release = fixture["release"]
    team = admin.ask("GET", f"/teams/{fixture['team']}")
    if release["teams"] == "unguarded":
        held = migrated_team(fixture["team_before"], team, fixture)
    elif release["teams"] == "guarded":
        held = kept_team(fixture["team_before"], team, fixture)
    else:
        raise RuntimeError(f"no team proof for the {release['teams']} team model")
    budgets = admin.ask("GET", f"/budgets/person/{fixture['person']}")
    zone = admin.ask("GET", "/configuration")["organisation"]["zone"]
    if release["budgets"] == "per_measure":
        effective = {"context_percent": fixture["first"], **fixture["original"]}
        if {row["measure"] for row in fixture["budgets_before"].get("budgets", [])} != set(effective):
            raise RuntimeError("legacy pending budget measures differ")
    elif release["budgets"] == "limits":
        effective = {}
        if fixture["budgets_before"].get("limits") != fixture["limits"]:
            raise RuntimeError("limits release's personal limits differ from those it was given")
    else:
        raise RuntimeError(f"no budget proof for the {release['budgets']} budget model")
    migrated_budgets(fixture["budgets_before"], budgets, zone, effective)
    return {"held_members": len(held), "unconfirmed_budgets": len(budgets["unconfirmed"])}
