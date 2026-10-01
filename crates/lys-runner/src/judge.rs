//! The tool-boundary judge: an agent's policy, installed with its launch,
//! applied to each tool call its harness asks about before the tool runs.
//!
//! A policy is deny constraints and nothing else. A call no rule matches is
//! passed back to the harness's own permission flow: the judge never
//! answers "allow", so it can never override a refusal the harness makes
//! itself. A hard rule always denies. A grantable rule stops denying only
//! when the live grant authority, asked at the moment of the call, permits
//! the session's bound agent the rule's action on its resource; an answer
//! that is unknown, unavailable or for another attempt denies, and no
//! earlier answer is kept to stand in for a later one.
//!
//! Only structured fields are read: `file_path` of Read, Write and Edit,
//! `path` of Glob and Grep, and the initial URL of `WebFetch`, whose hostname
//! is read with the URL parser. A relative path is resolved against the
//! session's bound working directory, dot segments are removed and every
//! existing component is resolved through its symbolic links before the
//! component-boundary prefix comparison, so `/p/denied-other` is not under
//! `/p/denied` and a link into a denied prefix is denied. A target that
//! cannot be resolved is refused, never guessed. Under a restrictive policy
//! (one with a path or host rule), a tool whose effects these fields cannot
//! name, a shell, an unknown MCP tool, a subagent's call, is denied
//! `policy_uninspectable`: shell text is never parsed as a security
//! authority. This is a tool-boundary check inside the harness, not process
//! containment: a redirect, a subprocess's own connection or a same-user
//! process that tampers with the harness is outside what it sees.

use std::collections::BTreeSet;
use std::path::{Component, Path, PathBuf};

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// The tools whose structured path field the judge reads, and the field.
pub const PATH_TOOLS: [(&str, &str); 5] = [
    ("Read", "file_path"),
    ("Write", "file_path"),
    ("Edit", "file_path"),
    ("Glob", "path"),
    ("Grep", "path"),
];

/// The tool whose initial URL's hostname the judge reads.
pub const HOST_TOOL: &str = "WebFetch";

/// A Lys resource a grantable rule names.
#[derive(
    Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, utoipa::ToSchema,
)]
#[serde(deny_unknown_fields)]
pub struct NamedResource {
    /// The resource's kind, as the grant model declares it.
    pub kind: String,
    /// The resource's id.
    pub id: String,
}

/// Who may lift a rule.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum Authority {
    /// No grant can allow it.
    Hard,
    /// It stops denying while the live grant authority permits the session's
    /// agent `action` on `resource`.
    Permission {
        /// The resource.
        resource: NamedResource,
        /// The action.
        action: String,
    },
}

/// What a rule's target is.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, utoipa::ToSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum RuleKind {
    /// The whole tool; no target.
    Tool,
    /// An absolute path prefix, compared component by component.
    PathPrefix,
    /// A canonical hostname.
    Host,
}

/// One deny rule.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct Rule {
    /// The rule's id, unique in its policy.
    pub id: String,
    /// The exact tool name it constrains.
    pub tool: String,
    /// What its target is.
    pub kind: RuleKind,
    /// The target: absent for a whole tool.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target: Option<String>,
    /// Who may lift it.
    pub authority: Authority,
}

/// An agent's policy, one immutable version of it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct Policy {
    /// The version, from 1; every decision names it.
    pub version: u64,
    /// The agent it constrains.
    pub agent: String,
    /// The deny rules, in order.
    pub rules: Vec<Rule>,
}

/// Why a policy was refused, by name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PolicyRefused {
    /// The refusal's name.
    pub refusal: &'static str,
    /// Why, in words.
    pub words: String,
}

fn refused(refusal: &'static str, words: String) -> PolicyRefused {
    PolicyRefused { refusal, words }
}

/// The field `tool` names its path in, when the judge reads one.
pub fn path_field(tool: &str) -> Option<&'static str> {
    PATH_TOOLS
        .iter()
        .find(|(name, _)| *name == tool)
        .map(|(_, field)| *field)
}

/// `host` in its canonical form: as the URL parser writes it, lowercase,
/// without a trailing dot; `None` when it is not a hostname alone.
pub fn canonical_host(host: &str) -> Option<String> {
    let parsed = url::Host::parse(host).ok()?;
    let text = parsed.to_string();
    let text = text.strip_suffix('.').unwrap_or(&text).to_ascii_lowercase();
    (!text.is_empty()).then_some(text)
}

impl Policy {
    /// Whether the policy has a path or host rule, so a tool the judge
    /// cannot inspect is denied rather than let past it.
    pub fn restrictive(&self) -> bool {
        self.rules.iter().any(|rule| rule.kind != RuleKind::Tool)
    }

    /// The policy, refused by name when its shape is wrong: an empty id or
    /// tool, a rule id used twice, a target where none belongs or missing
    /// where one does, a path that is not absolute and plain, a host that is
    /// not canonical, a path or host rule on a tool whose field is not read,
    /// or two rules on the same tool and target.
    pub fn checked(self) -> Result<Self, PolicyRefused> {
        if self.agent.is_empty() {
            return Err(refused(
                "policy_invalid",
                "a policy names its agent".to_owned(),
            ));
        }
        if self.version == 0 {
            return Err(refused(
                "policy_invalid",
                "a policy's version counts from 1".to_owned(),
            ));
        }
        let mut ids = BTreeSet::new();
        let mut targets = BTreeSet::new();
        for rule in &self.rules {
            check_rule(rule)?;
            if !ids.insert(rule.id.as_str()) {
                return Err(refused(
                    "policy_rule_duplicate",
                    format!("rule id `{}` is used twice", rule.id),
                ));
            }
            if !targets.insert((rule.tool.as_str(), rule.kind, rule.target.as_deref())) {
                return Err(refused(
                    "policy_target_ambiguous",
                    format!(
                        "rule `{}` names a tool and target another rule already names",
                        rule.id
                    ),
                ));
            }
        }
        Ok(self)
    }
}

fn check_rule(rule: &Rule) -> Result<(), PolicyRefused> {
    let name = &rule.id;
    if name.is_empty() || rule.tool.is_empty() {
        return Err(refused(
            "policy_invalid",
            "every rule has an id and names one exact tool".to_owned(),
        ));
    }
    if rule.tool.contains(['*', '?', ' ']) {
        return Err(refused(
            "policy_invalid",
            format!("rule `{name}` names a tool pattern; a rule names one exact tool"),
        ));
    }
    if let Authority::Permission { resource, action } = &rule.authority {
        if resource.kind.is_empty() || resource.id.is_empty() || action.is_empty() {
            return Err(refused(
                "policy_invalid",
                format!("rule `{name}` names a permission without its resource or action"),
            ));
        }
    }
    match (rule.kind, rule.target.as_deref()) {
        (RuleKind::Tool, None) => Ok(()),
        (RuleKind::Tool, Some(_)) => Err(refused(
            "policy_invalid",
            format!("rule `{name}` covers a whole tool and carries no target"),
        )),
        (RuleKind::PathPrefix | RuleKind::Host, None) => Err(refused(
            "policy_invalid",
            format!("rule `{name}` needs its target"),
        )),
        (RuleKind::PathPrefix, Some(target)) => {
            if path_field(&rule.tool).is_none() {
                return Err(refused(
                    "policy_target_uninspectable",
                    format!(
                        "rule `{name}`: the judge reads no path field of {}",
                        rule.tool
                    ),
                ));
            }
            if plain_absolute(target) {
                Ok(())
            } else {
                Err(refused(
                    "policy_target_ambiguous",
                    format!(
                        "rule `{name}`: a path prefix is absolute, without `.` or `..`, and ends without a slash"
                    ),
                ))
            }
        }
        (RuleKind::Host, Some(target)) => {
            if rule.tool != HOST_TOOL {
                return Err(refused(
                    "policy_target_uninspectable",
                    format!("rule `{name}`: only {HOST_TOOL}'s host is read"),
                ));
            }
            if canonical_host(target).as_deref() == Some(target) {
                Ok(())
            } else {
                Err(refused(
                    "policy_target_ambiguous",
                    format!("rule `{name}`: `{target}` is not a canonical hostname"),
                ))
            }
        }
    }
}

/// Whether `path` is absolute, holds no `.` or `..` and does not end in a
/// separator (the root alone excepted).
fn plain_absolute(path: &str) -> bool {
    let given = Path::new(path);
    given.is_absolute()
        && (path == "/" || !path.ends_with('/'))
        && !path.contains("//")
        && given
            .components()
            .all(|part| matches!(part, Component::RootDir | Component::Normal(_)))
}

/// A tool call as the judge reads it.
#[derive(Debug, Clone, Copy)]
pub struct Asked<'a> {
    /// The exact tool name.
    pub tool: &'a str,
    /// Its structured input.
    pub input: &'a Value,
    /// The session's bound working directory.
    pub cwd: &'a Path,
    /// Whether the call came from a subagent, whose session is not proved.
    pub subagent: bool,
}

/// A permission a grantable rule asks for.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Needed {
    /// The rule.
    pub rule: String,
    /// The resource.
    pub resource: NamedResource,
    /// The action.
    pub action: String,
}

/// What the judge made of a call.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Judgement {
    /// No rule applies: the harness's own permission flow decides.
    Pass,
    /// Denied, by name.
    Deny {
        /// The refusal: `policy_denied`, `policy_uninspectable` or
        /// `policy_target_ambiguous`.
        refusal: &'static str,
        /// The rule that denied it, when one did.
        rule: Option<String>,
        /// A safe summary of the target: a path, a host, or the tool alone.
        target: String,
        /// Why, in words.
        words: String,
    },
    /// Grantable rules matched: each permission is asked of the live grant
    /// authority, and any that is not given denies.
    Ask {
        /// The permissions, in rule order.
        needed: Vec<Needed>,
        /// A safe summary of the target.
        target: String,
    },
}

/// The target a call names, read from its structured fields.
enum Target {
    /// A resolved path.
    Path(PathBuf),
    /// A canonical host.
    Host(String),
    /// A field the judge reads that could not be resolved.
    Unresolved(String),
    /// No field the judge reads.
    Uninspectable,
}

fn target_of(asked: &Asked<'_>) -> Target {
    if let Some(field) = path_field(asked.tool) {
        let given = match asked.input.get(field) {
            None | Some(Value::Null) if field == "path" => ".",
            Some(Value::String(given)) => given.as_str(),
            _ => return Target::Unresolved(format!("{} carries no {field}", asked.tool)),
        };
        return match resolve(asked.cwd, given) {
            Ok(path) => Target::Path(path),
            Err(reason) => Target::Unresolved(reason),
        };
    }
    if asked.tool == HOST_TOOL {
        let Some(Value::String(given)) = asked.input.get("url") else {
            return Target::Unresolved(format!("{HOST_TOOL} carries no url"));
        };
        return match url::Url::parse(given) {
            Ok(parsed) if matches!(parsed.scheme(), "http" | "https") => {
                parsed.host_str().and_then(canonical_host).map_or_else(
                    || Target::Unresolved("the url names no host".to_owned()),
                    Target::Host,
                )
            }
            Ok(_) => Target::Unresolved("the url is not http or https".to_owned()),
            Err(error) => Target::Unresolved(format!("the url does not parse: {error}")),
        };
    }
    Target::Uninspectable
}

/// `given`, resolved against `cwd`: dot segments removed, every existing
/// component resolved through its links, the rest appended as written.
/// Refused, by reason, when it is empty, carries a NUL or a leading `~`,
/// has no absolute base, an existing component cannot be read, or a parent
/// traversal follows a missing component.
pub fn resolve(cwd: &Path, given: &str) -> Result<PathBuf, String> {
    if given.is_empty() || given.contains('\0') || given.starts_with('~') {
        return Err("the path is empty or not literal".to_owned());
    }
    let base = if Path::new(given).is_absolute() {
        PathBuf::from(given)
    } else if cwd.is_absolute() {
        cwd.join(given)
    } else {
        return Err("the session's working directory is not absolute".to_owned());
    };
    let mut resolved = PathBuf::from("/");
    let mut beyond = false;
    for part in base.components() {
        match part {
            Component::RootDir | Component::CurDir => {}
            Component::ParentDir => {
                if beyond {
                    return Err("the path traverses a parent after a missing component".to_owned());
                }
                resolved.pop();
            }
            Component::Normal(name) => {
                resolved.push(name);
                if beyond {
                    continue;
                }
                match std::fs::symlink_metadata(&resolved) {
                    Ok(_) => {
                        resolved = std::fs::canonicalize(&resolved).map_err(|error| {
                            format!("{} does not resolve: {error}", resolved.display())
                        })?;
                    }
                    Err(error) if error.kind() == std::io::ErrorKind::NotFound => beyond = true,
                    Err(error) => {
                        return Err(format!("{} cannot be read: {error}", resolved.display()));
                    }
                }
            }
            Component::Prefix(_) => return Err("the path carries a drive prefix".to_owned()),
        }
    }
    Ok(resolved)
}

fn summary(tool: &str, target: &Target) -> String {
    match target {
        Target::Path(path) => format!("{tool} {}", path.display()),
        Target::Host(host) => format!("{tool} host {host}"),
        Target::Unresolved(_) | Target::Uninspectable => tool.to_owned(),
    }
}

/// Whether `rule` covers the call to `tool` with `target`.
fn covers(rule: &Rule, tool: &str, target: &Target) -> Result<bool, String> {
    if rule.tool != tool {
        return Ok(false);
    }
    Ok(match (rule.kind, rule.target.as_deref(), target) {
        (RuleKind::Tool, _, _) => true,
        (RuleKind::PathPrefix, Some(prefix), Target::Path(path)) => {
            path.starts_with(resolve(Path::new("/"), prefix)?)
        }
        (RuleKind::Host, Some(host), Target::Host(named)) => host == named,
        _ => false,
    })
}

/// The judgement of `policy` on `asked`.
pub fn judge(policy: &Policy, asked: &Asked<'_>) -> Judgement {
    if policy.rules.is_empty() {
        return Judgement::Pass;
    }
    let restrictive = policy.restrictive();
    let target = target_of(asked);
    let shown = summary(asked.tool, &target);
    let deny = |refusal, rule: Option<&Rule>, words: String| Judgement::Deny {
        refusal,
        rule: rule.map(|rule| rule.id.clone()),
        target: shown.clone(),
        words,
    };
    let mut matched = Vec::new();
    for rule in &policy.rules {
        match covers(rule, asked.tool, &target) {
            Ok(true) => matched.push(rule),
            Ok(false) => {}
            Err(reason) => {
                return deny(
                    "policy_target_ambiguous",
                    Some(rule),
                    format!("rule `{}` has an unresolved path prefix: {reason}", rule.id),
                );
            }
        }
    }
    if let Some(hard) = matched
        .iter()
        .find(|rule| rule.authority == Authority::Hard)
    {
        return deny(
            "policy_denied",
            Some(*hard),
            format!(
                "rule `{}` of policy version {} denies {shown}; no grant can allow it",
                hard.id, policy.version
            ),
        );
    }
    if restrictive && asked.subagent {
        return deny(
            "policy_uninspectable",
            None,
            format!(
                "{} was asked by a subagent whose session is not proved to carry this policy",
                asked.tool
            ),
        );
    }
    match &target {
        Target::Uninspectable if restrictive => {
            return deny(
                "policy_uninspectable",
                None,
                format!(
                    "policy version {} constrains paths or hosts, and {} names none the judge can read",
                    policy.version, asked.tool
                ),
            );
        }
        Target::Unresolved(reason) if restrictive => {
            return deny(
                "policy_target_ambiguous",
                None,
                format!("the target of {} is not resolved: {reason}", asked.tool),
            );
        }
        _ => {}
    }
    let needed: Vec<Needed> = matched
        .iter()
        .filter_map(|rule| match &rule.authority {
            Authority::Permission { resource, action } => Some(Needed {
                rule: rule.id.clone(),
                resource: resource.clone(),
                action: action.clone(),
            }),
            Authority::Hard => None,
        })
        .collect();
    if needed.is_empty() {
        Judgement::Pass
    } else {
        Judgement::Ask {
            needed,
            target: shown,
        }
    }
}
