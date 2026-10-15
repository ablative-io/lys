impl Policy {
    /// Whether the policy has a path or host rule, so a tool the judge
    /// cannot inspect is denied rather than let past it.
    #[must_use]
    pub fn restrictive(&self) -> bool {
        self.rules.iter().any(|rule| rule.kind != RuleKind::Tool)
    }

    /// The policy, refused by name when its shape is wrong: an empty id or
    /// tool, a rule id used twice, a target where none belongs or missing
    /// where one does, a path that is not absolute and plain, a host that is
    /// not canonical, a path or host rule on a tool whose field is not read,
    /// or two rules on the same tool and target.
    ///
    /// # Errors
    /// - `PolicyRefused` if the agent, version, rule shape, literal path or hostname is invalid, or rule ids or targets repeat.
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
