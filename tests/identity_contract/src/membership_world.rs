//! A channel membership world (ACCESS-006): a seeded service, an approved
//! fixture app whose schema the test names, placements, people registered
//! through the API, root grants and their revocations, and the membership
//! routes asked with the app's credential, every request as a product
//! sends it. Nothing here decides membership: every answer is the
//! service's.

use std::error::Error;

use lys_identity_server::dev_seed::Seeded;
use serde_json::{Value, json};

use crate::apps::{Auth, approve, get, login, ok, op, post, registration, root, seeded};
use crate::harness::{ADMINISTRATOR, Service};

/// A started service with one approved fixture app.
pub struct World {
    /// The service.
    pub service: Service,
    /// The administrator's session cookie.
    pub admin: String,
    /// The fixture app's id, the prefix of its kinds.
    pub app: String,
    /// The app's credential, as a product holds it.
    pub credential: String,
    /// The grant log the service serves, as its answers name it.
    pub log: Value,
    /// The seeded people and their agents.
    pub seeded: Seeded,
}

/// The rows' `field` ids of a page, in order.
#[must_use]
pub fn ids(page: &Value, field: &str) -> Vec<String> {
    page["outcome"]["rows"]
        .as_array()
        .map(|rows| {
            rows.iter()
                .filter_map(|row| row[field]["id"].as_str().map(str::to_owned))
                .collect()
        })
        .unwrap_or_default()
}

/// The difference of each count between two readings of the counts route.
///
/// # Errors
/// A count missing from either reading.
pub fn delta(before: &Value, after: &Value) -> Result<Vec<(String, i128)>, Box<dyn Error>> {
    let members = after.as_object().ok_or("the counts are an object")?;
    let mut changed = Vec::new();
    for (name, value) in members {
        let now = value
            .as_u64()
            .ok_or_else(|| format!("{name} is not a count"))?;
        let was = before[name]
            .as_u64()
            .ok_or_else(|| format!("{name} was not counted before"))?;
        changed.push((name.clone(), i128::from(now) - i128::from(was)));
    }
    Ok(changed)
}

impl World {
    /// Seed the service, register `app` with `schema` as the administrator,
    /// approve it, and learn the grant log it serves from a refused question.
    ///
    /// # Errors
    /// Any step the service refuses.
    pub async fn open(app: &str, schema: &Value) -> Result<Self, Box<dyn Error>> {
        let (service, seeded) = seeded().await?;
        let admin = service.sign_in(login(ADMINISTRATOR)).await?;
        let body = registration(app, schema)?;
        ok(post(&service, "/apps", Auth::Cookie(&admin), &body).await?)?;
        approve(&service, &admin, app).await?;
        let credential = crate::app_custody::credential(app);
        let mut world = Self {
            service,
            admin,
            app: app.to_owned(),
            credential,
            log: json!({"identity": "not-this-log", "epoch": 0}),
            seeded,
        };
        world.log = world.served_log().await?;
        Ok(world)
    }

    /// The grant log the service serves, read from an admission it refuses
    /// for naming another log.
    ///
    /// # Errors
    /// The service answering anything but that refusal.
    pub async fn served_log(&self) -> Result<Value, Box<dyn Error>> {
        let probe = json!({
            "contract": 1,
            "log": {"identity": "not-this-log", "epoch": 0},
            "workspace": {"kind": self.kind("workspace"), "id": "probe"},
            "subject": {"id": self.seeded.people[0].id.to_string(), "kind": "person"},
            "at_least": 0,
        });
        let refused = self.ask("/admission", &probe).await?;
        if refused["verdict"]["refusal"] != "membership_log_mismatch" {
            return Err(format!("the probe was not refused for its log: {refused}").into());
        }
        Ok(refused["log"].clone())
    }

    /// The app's kind `name`.
    #[must_use]
    pub fn kind(&self, name: &str) -> String {
        format!("{}.{name}", self.app)
    }

    /// Place `child` under `parent`, both named by the app's short kinds.
    ///
    /// # Errors
    /// The service refusing the placement.
    pub async fn place(
        &self,
        child: (&str, &str),
        parent: (&str, &str),
        restricted: bool,
    ) -> Result<(), Box<dyn Error>> {
        let body = json!({
            "operation": op()?,
            "child": {"kind": self.kind(child.0), "id": child.1},
            "parent": {"kind": self.kind(parent.0), "id": parent.1},
            "restricted": restricted,
        });
        let path = format!("/apps/{}/placements", self.app);
        ok(post(&self.service, &path, Auth::Cookie(&self.admin), &body).await?)?;
        Ok(())
    }

    /// A root grant of `relation` on the app's `(kind, id)` to `holder`,
    /// answering the issue.
    ///
    /// # Errors
    /// The service refusing the grant.
    pub async fn grant(
        &self,
        holder: &str,
        (kind, id): (&str, &str),
        relation: &str,
    ) -> Result<Value, Box<dyn Error>> {
        let kind = self.kind(kind);
        ok(root(&self.service, &self.admin, holder, (&kind, id), relation).await?)
    }

    /// A grant of `relation` on the app's `kind` `id`, held by `agent`:
    /// a root for `person` that may pass `actions` to agents, passed on to
    /// the agent. Answers the passed-on grant's issue.
    ///
    /// # Errors
    /// The service refusing either grant.
    pub async fn passed_to_agent(
        &self,
        person: &str,
        agent: &str,
        (kind, id): (&str, &str),
        (relation, actions): (&str, &[String]),
    ) -> Result<Value, Box<dyn Error>> {
        let resource = json!({"kind": self.kind(kind), "id": id});
        let admin = Auth::Cookie(&self.admin);
        let body = json!({
            "operation": op()?, "route": "api", "holder": person,
            "resource": resource, "relation": relation,
            "pass_on": {"kind": "to", "actions": actions, "recipients": ["agent"]},
            "window": {"starts_at": 0, "ends_at": null},
        });
        let source = ok(post(&self.service, "/grants/roots", admin, &body).await?)?;
        let body = json!({
            "operation": op()?, "route": "api", "source": source["grant"],
            "recipient": agent, "responsible": person, "resource": resource,
            "relation": relation, "pass_on": {"kind": "use_only"},
            "window": {"starts_at": 0, "ends_at": null},
        });
        ok(post(&self.service, "/grants", admin, &body).await?)
    }

    /// Revoke the grant `issued` names, answering the revocation.
    ///
    /// # Errors
    /// The service refusing the revocation.
    pub async fn revoke(&self, issued: &Value) -> Result<Value, Box<dyn Error>> {
        let grant = issued["grant"]
            .as_str()
            .ok_or("the issue names its grant")?;
        let body = json!({"operation": op()?, "route": "api", "reason": "left the ward"});
        let path = format!("/grants/{grant}/revoke");
        ok(post(&self.service, &path, Auth::Cookie(&self.admin), &body).await?)
    }

    /// Register a person `name`, bind `subject` at the issuer to them and
    /// activate them, answering their id.
    ///
    /// # Errors
    /// The service refusing any step.
    pub async fn person(&self, name: &str, subject: &str) -> Result<String, Box<dyn Error>> {
        let admin = Auth::Cookie(&self.admin);
        let body = json!({"operation": op()?, "display_name": name});
        let made = ok(post(&self.service, "/people", admin, &body).await?)?;
        let person = made["person"].as_str().ok_or("no person")?.to_owned();
        let body = json!({
            "operation": op()?,
            "issuer": self.service.issuer.issuer(),
            "subject": subject,
        });
        let path = format!("/people/{person}/logins");
        ok(post(&self.service, &path, admin, &body).await?)?;
        let body = json!({"operation": op()?, "transition": "activate", "reason": "test"});
        let path = format!("/identities/{person}/transitions");
        ok(post(&self.service, &path, admin, &body).await?)?;
        Ok(person)
    }

    /// Ask the membership route `/grants/membership{path}` with the app's
    /// credential, answering its body.
    ///
    /// # Errors
    /// Any status but 200.
    pub async fn ask(&self, path: &str, body: &Value) -> Result<Value, Box<dyn Error>> {
        let path = format!("/grants/membership{path}");
        ok(post(&self.service, &path, Auth::Bearer(&self.credential), body).await?)
    }

    /// The capability's counts, read by the administrator.
    ///
    /// # Errors
    /// Any status but 200.
    pub async fn counts(&self) -> Result<Value, Box<dyn Error>> {
        ok(get(
            &self.service,
            "/grants/membership/counts",
            Auth::Cookie(&self.admin),
        )
        .await?)
    }

    /// A single decision request: `subject` (id, kind) taking `action` on
    /// the app's channel `channel` in workspace `workspace`.
    #[must_use]
    pub fn decision(
        &self,
        workspace: &str,
        subject: (&str, &str),
        channel: &str,
        action: &str,
    ) -> Value {
        json!({
            "contract": 1,
            "log": self.log,
            "workspace": {"kind": self.kind("workspace"), "id": workspace},
            "subject": {"id": subject.0, "kind": subject.1},
            "resource": {"kind": self.kind("channel"), "id": channel},
            "action": action,
            "at_least": 0,
        })
    }

    /// An admission request for `subject` (id, kind) to `workspace`.
    #[must_use]
    pub fn admission(&self, workspace: &str, subject: (&str, &str)) -> Value {
        json!({
            "contract": 1,
            "log": self.log,
            "workspace": {"kind": self.kind("workspace"), "id": workspace},
            "subject": {"id": subject.0, "kind": subject.1},
            "at_least": 0,
        })
    }

    /// A page of the channels `subject` (id, kind) may read in `workspace`.
    #[must_use]
    pub fn resources(
        &self,
        workspace: &str,
        subject: (&str, &str),
        rows: u32,
        after: &Value,
    ) -> Value {
        json!({
            "contract": 1,
            "log": self.log,
            "workspace": {"kind": self.kind("workspace"), "id": workspace},
            "subject": {"id": subject.0, "kind": subject.1},
            "kind": self.kind("channel"),
            "action": "read",
            "at_least": 0,
            "bounds": {"rows": rows, "bytes": 65_536},
            "after": after,
        })
    }

    /// A page of the subjects who may read the channel `channel` in
    /// `workspace`.
    #[must_use]
    pub fn recipients(&self, workspace: &str, channel: &str, rows: u32, after: &Value) -> Value {
        json!({
            "contract": 1,
            "log": self.log,
            "workspace": {"kind": self.kind("workspace"), "id": workspace},
            "resource": {"kind": self.kind("channel"), "id": channel},
            "action": "read",
            "at_least": 0,
            "bounds": {"rows": rows, "bytes": 65_536},
            "after": after,
        })
    }
}

/// An estate above its workspaces, workspaces whose members read their
/// channels, and channels whose read and post are held by separate
/// relations, all under `app`'s prefix.
#[must_use]
pub fn ward_schema(app: &str) -> Value {
    json!({"kinds": {
        format!("{app}.estate"): {
            "actions": ["read", "post"],
            "relations": {"steward": ["read"]},
            "parents": []
        },
        format!("{app}.workspace"): {
            "actions": ["read", "post"],
            "relations": {"member": ["read"]},
            "parents": [format!("{app}.estate")]
        },
        format!("{app}.channel"): {
            "actions": ["read", "post"],
            "relations": {"reader": ["read"], "poster": ["post"]},
            "parents": [format!("{app}.workspace")]
        }
    }})
}
