#![cfg(test)]
//! `/canvas`: a person's arrangement and saved layouts are kept for them,
//! one file each, replaced whole; they are theirs alone; and keeping one
//! wakes no one else's page.

use std::error::Error;
use std::path::PathBuf;

use identity_contract::apps::{Auth, get, ok, post, put, refused};
use identity_contract::fake_issuer::Login;
use identity_contract::harness::{ADMINISTRATOR, Service};
use lys_identity_server::dev_seed::seed_configured;
use serde_json::{Value, json};

type Result<T = ()> = std::result::Result<T, Box<dyn Error>>;

const BEA: &str = "bea-subject";

fn login(subject: &str) -> Login {
    Login {
        subject: subject.to_owned(),
        email: "shared@example.test".to_owned(),
    }
}

/// The service with Ada and Bea signed in, and the folder its canvases are kept in.
async fn table() -> Result<(Service, String, String, PathBuf)> {
    let (service, folder) = Service::start_with(|config| {
        seed_configured(config, [ADMINISTRATOR, BEA])?;
        Ok(config.log_dir.with_file_name("canvas"))
    })
    .await?;
    let ada = service.sign_in(login(ADMINISTRATOR)).await?;
    let bea = service.sign_in(login(BEA)).await?;
    Ok((service, ada, bea, folder))
}

/// An arrangement with one agent's window at `x`, a labelled box, a note and a line between them.
fn arrangement(x: f64) -> Value {
    json!({
        "boxes": {"agent:one": {"x": x, "y": 40.0, "w": 760.0, "h": 480.0}},
        "open": ["agent:one"],
        "groups": [{"id": "group:a", "label": "Iridium", "colour": "teal", "x": 0.0, "y": 0.0, "w": 900.0, "h": 600.0}],
        "notes": [{"id": "note:a", "text": "Ask about the build", "x": 20.0, "y": 540.0, "w": 260.0, "h": 180.0}],
        "links": [
            {"id": "link:a", "from": "agent:one", "to": "note:a"},
            {"id": "link:b", "from": "note:a", "to": "group:a", "from_side": "top", "to_side": "left"},
            {"id": "link:c", "from": "agent:one", "to": "widget:a", "from_side": "right", "to_side": "left"},
        ],
        "widgets": [
            {"id": "widget:a", "kind": "usage", "shows": "window/300", "view": "settings", "look": "dial", "locked": true, "colour": "blue", "x": 820.0, "y": 40.0, "w": 340.0, "h": 250.0},
            {"id": "widget:b", "kind": "goals", "x": 820.0, "y": 90.0, "w": 340.0, "h": 190.0},
            {"id": "widget:c", "kind": "usage", "faces": ["dollars/day", "window/10080"], "view": "faces", "x": 820.0, "y": 140.0, "w": 340.0, "h": 120.0},
            {"id": "widget:d", "kind": "usage", "faces": [], "x": 820.0, "y": 190.0, "w": 340.0, "h": 120.0},
        ],
    })
}

fn names(layouts: &Value) -> Result<Vec<String>> {
    layouts["layouts"]
        .as_array()
        .ok_or("no layouts")?
        .iter()
        .map(|layout| {
            layout["name"]
                .as_str()
                .map(str::to_owned)
                .ok_or_else(|| "a layout has no name".into())
        })
        .collect()
}

#[tokio::test]
async fn a_persons_canvas_is_kept_whole_for_them_and_is_theirs_alone() -> Result {
    let (service, ada, bea, folder) = table().await?;
    let empty = json!({"arrangement": null, "layouts": []});
    assert_eq!(
        ok(get(&service, "/canvas", Auth::Cookie(&ada)).await?)?,
        empty
    );
    assert!(!folder.exists(), "reading a canvas makes nothing");

    let kept = ok(put(
        &service,
        "/canvas",
        Auth::Cookie(&ada),
        &json!({"arrangement": arrangement(100.0)}),
    )
    .await?)?;
    assert_eq!(kept["arrangement"], arrangement(100.0));
    assert_eq!(
        ok(get(&service, "/canvas", Auth::Cookie(&ada)).await?)?,
        json!({"arrangement": arrangement(100.0), "layouts": []})
    );
    // Bea's canvas is her own: Ada's change is not in it, and hers does not touch Ada's.
    assert_eq!(
        ok(get(&service, "/canvas", Auth::Cookie(&bea)).await?)?,
        empty
    );
    ok(put(
        &service,
        "/canvas",
        Auth::Cookie(&bea),
        &json!({"arrangement": arrangement(900.0)}),
    )
    .await?)?;
    assert_eq!(
        ok(get(&service, "/canvas", Auth::Cookie(&ada)).await?)?["arrangement"],
        arrangement(100.0)
    );

    // Kept again and again, a person still has one file, and nothing is left beside it.
    for x in [110.0, 120.0, 130.0] {
        ok(put(
            &service,
            "/canvas",
            Auth::Cookie(&ada),
            &json!({"arrangement": arrangement(x)}),
        )
        .await?)?;
    }
    let mut files = std::fs::read_dir(&folder)?
        .map(|entry| Ok(entry?.file_name().to_string_lossy().into_owned()))
        .collect::<Result<Vec<String>>>()?;
    files.sort();
    assert_eq!(files.len(), 2, "one file for each person: {files:?}");
    assert!(
        files.iter().all(|name| name.starts_with("person-")
            && std::path::Path::new(name)
                .extension()
                .is_some_and(|extension| extension == "json")),
        "{files:?}"
    );
    let on_disk: Value = serde_json::from_slice(&std::fs::read(folder.join(&files[0]))?)?;
    assert!(
        on_disk["arrangement"] == arrangement(130.0)
            || on_disk["arrangement"] == arrangement(900.0),
        "a person's file holds their latest canvas whole: {on_disk}"
    );
    Ok(())
}

#[tokio::test]
async fn layouts_are_saved_by_name_saved_over_in_place_and_removed() -> Result {
    let (service, ada, bea, _folder) = table().await?;
    let save = |name: &str, x: f64| json!({"name": name, "arrangement": arrangement(x)});
    let first = ok(post(
        &service,
        "/canvas/layouts",
        Auth::Cookie(&ada),
        &save("Morning", 1.0),
    )
    .await?)?;
    assert_eq!(names(&first)?, ["Morning"]);
    let second = ok(post(
        &service,
        "/canvas/layouts",
        Auth::Cookie(&ada),
        &save(" Evening ", 2.0),
    )
    .await?)?;
    assert_eq!(names(&second)?, ["Morning", "Evening"]);
    // Saved over, Morning keeps its place and holds the new arrangement.
    let over = ok(post(
        &service,
        "/canvas/layouts",
        Auth::Cookie(&ada),
        &save("Morning", 3.0),
    )
    .await?)?;
    assert_eq!(names(&over)?, ["Morning", "Evening"]);
    assert_eq!(over["layouts"][0]["arrangement"], arrangement(3.0));
    assert!(over["layouts"][0]["saved_at"].as_u64().is_some());
    // The arrangement being worked in is not a layout, and a layout is not the arrangement.
    let whole = ok(get(&service, "/canvas", Auth::Cookie(&ada)).await?)?;
    assert_eq!(whole["arrangement"], Value::Null);
    assert_eq!(names(&whole)?, ["Morning", "Evening"]);
    assert_eq!(
        names(&ok(get(&service, "/canvas", Auth::Cookie(&bea)).await?)?)?,
        Vec::<String>::new()
    );

    // Bea cannot remove Ada's layout: she has none of that name.
    refused(
        &post(
            &service,
            "/canvas/layouts/remove",
            Auth::Cookie(&bea),
            &json!({"name": "Morning"}),
        )
        .await?,
        400,
        "CanvasRefused",
    )?;
    let after = ok(post(
        &service,
        "/canvas/layouts/remove",
        Auth::Cookie(&ada),
        &json!({"name": "Morning"}),
    )
    .await?)?;
    assert_eq!(names(&after)?, ["Evening"]);
    refused(
        &post(
            &service,
            "/canvas/layouts/remove",
            Auth::Cookie(&ada),
            &json!({"name": "Morning"}),
        )
        .await?,
        400,
        "CanvasRefused",
    )?;
    Ok(())
}

#[tokio::test]
async fn what_cannot_be_kept_is_refused_by_name_and_changes_nothing() -> Result {
    let (service, ada, _bea, _folder) = table().await?;
    let good = json!({"arrangement": arrangement(5.0)});
    ok(put(&service, "/canvas", Auth::Cookie(&ada), &good).await?)?;
    refused(
        &put(&service, "/canvas", Auth::Nobody, &good).await?,
        401,
        "NotSignedIn",
    )?;
    refused(
        &get(&service, "/canvas", Auth::Nobody).await?,
        401,
        "NotSignedIn",
    )?;
    for name in ["", "   ", "two\nlines"] {
        refused(
            &post(
                &service,
                "/canvas/layouts",
                Auth::Cookie(&ada),
                &json!({"name": name, "arrangement": arrangement(1.0)}),
            )
            .await?,
            400,
            "CanvasRefused",
        )?;
    }
    // A body that is not an arrangement: a member nobody declared, a place that is not a number, no arrangement at all.
    let mut unknown = arrangement(1.0);
    unknown["colour"] = json!("red");
    let mut worded = arrangement(1.0);
    worded["boxes"]["agent:one"]["x"] = json!("left");
    let mut sideways = arrangement(1.0);
    sideways["links"][1]["from_side"] = json!("up");
    // A widget in a view, or drawn a way, that there is not; and one with no kind.
    let mut viewed = arrangement(1.0);
    viewed["widgets"][0]["view"] = json!("inside out");
    let mut drawn = arrangement(1.0);
    drawn["widgets"][0]["look"] = json!("pie");
    let mut kindless = arrangement(1.0);
    kindless["widgets"][1] = json!({"id": "widget:b", "x": 1.0, "y": 1.0, "w": 1.0, "h": 1.0});
    for body in [
        json!({"arrangement": unknown}),
        json!({"arrangement": worded}),
        json!({"arrangement": sideways}),
        json!({"arrangement": viewed}),
        json!({"arrangement": drawn}),
        json!({"arrangement": kindless}),
        json!({}),
    ] {
        refused(
            &put(&service, "/canvas", Auth::Cookie(&ada), &body).await?,
            400,
            "RequestMalformed",
        )?;
    }
    assert_eq!(
        ok(get(&service, "/canvas", Auth::Cookie(&ada)).await?)?,
        json!({"arrangement": arrangement(5.0), "layouts": []})
    );
    Ok(())
}

#[tokio::test]
async fn keeping_a_canvas_wakes_no_page() -> Result {
    let (service, ada, _bea, _folder) = table().await?;
    let before = ok(get(&service, "/changes", Auth::Cookie(&ada)).await?)?;
    ok(put(
        &service,
        "/canvas",
        Auth::Cookie(&ada),
        &json!({"arrangement": arrangement(7.0)}),
    )
    .await?)?;
    ok(post(
        &service,
        "/canvas/layouts",
        Auth::Cookie(&ada),
        &json!({"name": "Morning", "arrangement": arrangement(7.0)}),
    )
    .await?)?;
    let after = ok(get(&service, "/changes", Auth::Cookie(&ada)).await?)?;
    assert_eq!(
        before["generation"], after["generation"],
        "a person's canvas is their own picture: keeping it changes nothing another page reads"
    );
    Ok(())
}
