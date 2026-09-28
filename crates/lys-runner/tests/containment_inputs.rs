//! Native input checks reject aliasing, extra hard links and replaced ancestors before launch.

use std::error::Error;
use std::path::Path;

use lys_runner::containment_inputs::Inputs;
use lys_runner::containment_policy::{Binding, Capability, NativePolicy, Plan};
use lys_runner::judge::Policy;

type TestResult = Result<(), Box<dyn Error>>;

fn fixture(base: &Path) -> Result<Plan, Box<dyn Error>> {
    for name in ["home", "workspace", "runtime", "protected"] {
        std::fs::create_dir(base.join(name))?;
    }
    let policy = NativePolicy {
        tools: Policy {
            version: 1,
            agent: "test".to_owned(),
            rules: Vec::new(),
        },
        home: base.join("home"),
        workspace: base.join("workspace"),
        runtime_reads: [base.join("runtime")].into(),
        protected: [base.join("protected")].into(),
        allowed_hosts: [].into(),
        denied_hosts: [].into(),
        required: [
            Capability::Filesystem,
            Capability::HostnameEgress,
            Capability::KernelAudit,
        ]
        .into(),
    };
    Ok(Plan {
        binding: Binding {
            runner: "runner".to_owned(),
            session: "session".to_owned(),
            incarnation: "life".to_owned(),
            agent: "test".to_owned(),
            revision: 1,
            digest: policy.digest()?,
        },
        policy,
    })
}

#[test]
fn held_inputs_recheck_successfully_without_changes() -> TestResult {
    let temporary = tempfile::tempdir()?;
    let plan = fixture(&temporary.path().canonicalize()?)?;
    Inputs::open(&plan, &plan.binding)?.verify(&plan, &plan.binding)?;
    Ok(())
}

#[test]
fn symlink_aliases_cannot_hide_runtime_or_protected_inputs_in_workspace() -> TestResult {
    for protected in [false, true] {
        let temporary = tempfile::tempdir()?;
        let base = temporary.path().canonicalize()?;
        let mut plan = fixture(&base)?;
        let alias = base.join("alias");
        std::os::unix::fs::symlink(&plan.policy.workspace, &alias)?;
        let input = alias.join("secret");
        std::fs::write(plan.policy.workspace.join("secret"), "fixture")?;
        if protected {
            plan.policy.protected = [input.clone()].into();
        } else {
            plan.policy.runtime_reads = [input.clone()].into();
        }
        plan.binding.digest = plan.policy.digest()?;
        let error = Inputs::open(&plan, &plan.binding)
            .err()
            .ok_or("alias accepted")?;
        assert_eq!(error.name(), "containment_input_unavailable");
        assert!(
            error
                .to_string()
                .contains(input.to_str().ok_or("fixture UTF-8")?)
        );
    }
    Ok(())
}

#[test]
fn protected_file_with_a_writable_hard_link_is_refused() -> TestResult {
    let temporary = tempfile::tempdir()?;
    let base = temporary.path().canonicalize()?;
    let mut plan = fixture(&base)?;
    let secret = base.join("protected/secret");
    std::fs::write(&secret, "fixture")?;
    std::fs::hard_link(&secret, plan.policy.workspace.join("alias"))?;
    plan.policy.protected = [secret].into();
    plan.binding.digest = plan.policy.digest()?;
    let error = Inputs::open(&plan, &plan.binding)
        .err()
        .ok_or("hard link accepted")?;
    assert!(error.to_string().contains("additional hard links"));
    Ok(())
}

#[test]
fn replaced_runtime_object_is_refused_at_recheck() -> TestResult {
    let temporary = tempfile::tempdir()?;
    let base = temporary.path().canonicalize()?;
    let plan = fixture(&base)?;
    let inputs = Inputs::open(&plan, &plan.binding)?;
    std::fs::rename(base.join("runtime"), base.join("old-runtime"))?;
    std::fs::create_dir(base.join("runtime"))?;
    let error = inputs
        .verify(&plan, &plan.binding)
        .err()
        .ok_or("replacement accepted")?;
    assert!(error.to_string().contains("identity or ancestry changed"));
    Ok(())
}

#[test]
fn file_cannot_become_a_writable_directory_root() -> TestResult {
    let temporary = tempfile::tempdir()?;
    let base = temporary.path().canonicalize()?;
    let plan = fixture(&base)?;
    std::fs::remove_dir(&plan.policy.home)?;
    std::fs::write(&plan.policy.home, "not a directory")?;
    let error = Inputs::open(&plan, &plan.binding)
        .err()
        .ok_or("file root accepted")?;
    assert!(error.to_string().contains("not a directory"));
    Ok(())
}

#[cfg(target_os = "macos")]
#[test]
fn private_tmp_alias_is_refused_instead_of_bypassing_protected_overlap() -> TestResult {
    let temporary = tempfile::tempdir_in("/private/tmp")?;
    let base = temporary.path().canonicalize()?;
    let mut plan = fixture(&base)?;
    let secret = plan.policy.workspace.join("audit");
    std::fs::write(&secret, "fixture")?;
    let alias = Path::new("/").join(secret.strip_prefix("/private")?);
    plan.policy.protected = [alias.clone()].into();
    plan.binding.digest = plan.policy.digest()?;
    let error = Inputs::open(&plan, &plan.binding)
        .err()
        .ok_or("tmp alias accepted")?;
    assert_eq!(error.name(), "containment_input_unavailable");
    assert!(
        error
            .to_string()
            .contains(alias.to_str().ok_or("fixture UTF-8")?)
    );
    Ok(())
}
