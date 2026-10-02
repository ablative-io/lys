#![cfg(test)]

use std::error::Error;
use std::path::Path;

use axum::Router;
use axum::routing::get;
use identity_contract::harness::{GRANT_MODEL, Service};
use serde_json::Value;

use super::{cache_control, content_type, serving};

async fn started(dir: &Path) -> Result<String, Box<dyn Error>> {
    let api = Router::new().route("/authority", get(|| async { "the api" }));
    let app = serving(dir, api)?;
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
    let base = format!("http://{}", listener.local_addr()?);
    tokio::spawn(async move { axum::serve(listener, app).await });
    Ok(base)
}

#[tokio::test]
async fn the_page_the_assets_and_the_api_share_one_origin() -> Result<(), Box<dyn Error>> {
    let dir = tempfile::TempDir::new()?;
    std::fs::write(dir.path().join("index.html"), "<html>screens</html>")?;
    std::fs::create_dir(dir.path().join("assets"))?;
    std::fs::write(dir.path().join("assets").join("app.js"), "console.log(1)")?;
    let base = started(dir.path()).await?;
    let client = reqwest::Client::new();

    let page = client.get(format!("{base}/")).send().await?;
    assert_eq!(page.status(), 200);
    assert_eq!(
        page.headers()["content-type"].to_str()?,
        "text/html; charset=utf-8"
    );
    assert_eq!(page.text().await?, "<html>screens</html>");

    let asset = client.get(format!("{base}/assets/app.js")).send().await?;
    assert_eq!(asset.status(), 200);
    assert_eq!(
        asset.headers()["cache-control"].to_str()?,
        "public, max-age=31536000, immutable"
    );
    assert_eq!(asset.text().await?, "console.log(1)");

    let api = client.get(format!("{base}/api/authority")).send().await?;
    assert_eq!(api.status(), 200);
    assert_eq!(api.text().await?, "the api");

    let screen = client.get(format!("{base}/people/abc")).send().await?;
    assert_eq!(screen.status(), 200);
    assert_eq!(screen.text().await?, "<html>screens</html>");

    let missing = client.get(format!("{base}/assets/gone.js")).send().await?;
    assert_eq!(missing.status(), 404);
    Ok(())
}

#[tokio::test]
async fn installed_bytes_remain_served_without_reading_the_files_again()
-> Result<(), Box<dyn Error>> {
    let dir = tempfile::TempDir::new()?;
    std::fs::write(dir.path().join("index.html"), "installed page")?;
    std::fs::create_dir(dir.path().join("assets"))?;
    std::fs::write(dir.path().join("assets/app.js"), "installed asset")?;
    let base = started(dir.path()).await?;
    std::fs::remove_file(dir.path().join("index.html"))?;
    std::fs::remove_file(dir.path().join("assets/app.js"))?;
    let client = reqwest::Client::new();
    for (path, expected) in [
        ("/", "installed page"),
        ("/assets/app.js", "installed asset"),
    ] {
        let answer = client.get(format!("{base}{path}")).send().await?;
        assert_eq!(answer.status(), 200, "{path}");
        assert_eq!(answer.text().await?, expected);
    }
    Ok(())
}

#[tokio::test]
async fn a_matching_installed_etag_needs_no_body() -> Result<(), Box<dyn Error>> {
    let dir = tempfile::TempDir::new()?;
    std::fs::write(dir.path().join("index.html"), "installed page")?;
    let base = started(dir.path()).await?;
    let client = reqwest::Client::new();
    let page = client.get(format!("{base}/")).send().await?;
    let tag = page
        .headers()
        .get(reqwest::header::ETAG)
        .ok_or("no installed ETag")?
        .clone();
    let answer = client
        .get(format!("{base}/"))
        .header(reqwest::header::IF_NONE_MATCH, tag)
        .send()
        .await?;
    assert_eq!(answer.status(), 304);
    assert!(answer.bytes().await?.is_empty());
    Ok(())
}

#[tokio::test]
async fn a_path_outside_the_directory_is_never_read() -> Result<(), Box<dyn Error>> {
    let dir = tempfile::TempDir::new()?;
    let inside = dir.path().join("served");
    std::fs::create_dir(&inside)?;
    std::fs::write(inside.join("index.html"), "<html></html>")?;
    std::fs::write(dir.path().join("outside.txt"), "private")?;
    let base = started(&inside).await?;
    let client = reqwest::Client::new();
    let answer = client
        .get(format!("{base}/assets/..%2F..%2Foutside.txt"))
        .send()
        .await?;
    assert_eq!(answer.status(), 404);
    let answer = client
        .get(format!("{base}/..%2Foutside.txt"))
        .send()
        .await?;
    assert_eq!(answer.status(), 404);
    Ok(())
}

#[tokio::test]
async fn without_the_page_the_root_says_what_to_run() -> Result<(), Box<dyn Error>> {
    let dir = tempfile::TempDir::new()?;
    let base = started(dir.path()).await?;
    let answer = reqwest::get(format!("{base}/")).await?;
    assert_eq!(answer.status(), 404);
    assert!(answer.text().await?.contains("lys identity install"));
    Ok(())
}

const PAGE: &str = "<html>the screens</html>";

/// The whole service, serving the screens in `surface` when one is given.
async fn whole(surface: Option<&Path>) -> Result<Service, Box<dyn Error>> {
    let served = surface.map(Path::to_path_buf);
    let (service, ()) = Service::start_adjusted(
        GRANT_MODEL,
        None,
        None,
        None,
        move |config| config.surface_dir = served,
        |_| Ok(()),
    )
    .await?;
    Ok(service)
}

/// The status and the body text `path` is answered with.
async fn text(service: &Service, path: &str) -> Result<(u16, String), Box<dyn Error>> {
    let answer = reqwest::get(format!("{}{path}", service.base)).await?;
    Ok((answer.status().as_u16(), answer.text().await?))
}

#[tokio::test]
async fn an_unknown_api_path_is_refused_by_name_never_the_page() -> Result<(), Box<dyn Error>> {
    let dir = tempfile::TempDir::new()?;
    std::fs::write(dir.path().join("index.html"), PAGE)?;
    let service = whole(Some(dir.path())).await?;

    let (status, body) = text(&service, "/api/health-unknown").await?;
    assert_eq!(status, 404);
    assert!(!body.contains(PAGE));
    let refusal: Value = serde_json::from_str(&body)?;
    assert_eq!(refusal["refusal"], "NotAnApiRoute");
    assert_eq!(refusal["error"], "/api/health-unknown is not an API route");

    let (status, body) = text(&service, "/api/authority").await?;
    assert_eq!(status, 200);
    let authority: Value = serde_json::from_str(&body)?;
    assert_eq!(authority["authority"], crate::admission::AUTHORITY);

    let (status, body) = text(&service, "/people").await?;
    assert_eq!(status, 200);
    assert_eq!(body, PAGE);

    let (status, body) = text(&service, "/health").await?;
    assert_eq!(status, 200);
    assert_eq!(body, PAGE);

    let (status, body) = text(&service, "/assets/missing.js").await?;
    assert_eq!(status, 404);
    assert!(!body.contains(PAGE));
    Ok(())
}

#[tokio::test]
async fn without_the_screens_an_unknown_path_is_refused_alike() -> Result<(), Box<dyn Error>> {
    let service = whole(None).await?;
    let (status, body) = text(&service, "/health-unknown").await?;
    assert_eq!(status, 404);
    let refusal: Value = serde_json::from_str(&body)?;
    assert_eq!(refusal["refusal"], "NotAnApiRoute");
    assert_eq!(refusal["error"], "/health-unknown is not an API route");
    Ok(())
}

#[test]
fn media_types_and_caching_follow_the_path() {
    assert_eq!(content_type(Path::new("a.css")), "text/css; charset=utf-8");
    assert_eq!(content_type(Path::new("a.woff2")), "font/woff2");
    assert_eq!(
        content_type(Path::new("a.unknown")),
        "application/octet-stream"
    );
    assert_eq!(cache_control(Path::new("index.html")), "no-cache");
    assert_eq!(
        cache_control(Path::new("assets/x.js")),
        "public, max-age=31536000, immutable"
    );
}
