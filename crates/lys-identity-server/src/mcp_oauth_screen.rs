//! The consent form lists only permissions the person may pass on.
use super::{Apps, escaped};
use crate::ServerError;
use axum::http::header;
use axum::response::{Html, IntoResponse, Response};

pub(super) fn consent_screen(
    apps: &Apps,
    person: lys_identity::PersonId,
    asking: &str,
    name: &str,
    redirect_uri: &str,
) -> Result<Response, ServerError> {
    let mut choices = String::new();
    for offer in
        crate::mcp_oauth_grants::grouped(crate::mcp_oauth_grants::offers(&apps.state, person)?)?
    {
        choices.push_str("<label><input type=\"checkbox\" name=\"grant\" value=\"");
        choices.push_str(&escaped(&offer.key));
        choices.push_str("\"> ");
        choices.push_str(&escaped(&offer.words()));
        choices.push_str("</label><br>");
    }
    let choices = if choices.is_empty() {
        "<p>You hold nothing you can pass on to an agent yet, so it starts with no permissions.</p>"
            .to_owned()
    } else {
        format!("<p>Choose what it may do for you:</p><p>{choices}</p>")
    };
    let name = escaped(name);
    let site = escaped(
        &reqwest::Url::parse(redirect_uri)
            .ok()
            .and_then(|url| url.host_str().map(str::to_owned))
            .unwrap_or_default(),
    );
    Ok((
        [
            (header::CACHE_CONTROL, "no-store"),
            (header::X_FRAME_OPTIONS, "DENY"),
            (header::CONTENT_SECURITY_POLICY, "default-src 'none'; style-src 'unsafe-inline'; form-action 'self' https: http:"),
        ],
        Html(format!(
            "<!doctype html><html lang=\"en\"><head><meta charset=\"utf-8\">\
<meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">\
<title>Connect {name} to Lys</title>\
<style>body{{font-family:system-ui,sans-serif;max-width:32rem;margin:4rem auto;padding:0 1.5rem;line-height:1.5;color:#1d1d1f}}\
button{{font:inherit;padding:.6rem 1.2rem;margin-right:.75rem;border-radius:.5rem;border:1px solid #888;background:#fff;cursor:pointer}}\
button[value=approve]{{background:#1d1d1f;color:#fff;border-color:#1d1d1f}}</style></head><body>\
<h1>Connect {name}?</h1>\
<p>{name}, at {site}, is asking to act for you in Lys.</p>\
<p>If you connect it, Lys adds it as one of your agents. It can do only what you choose here, \
and you can take that away or remove it at any time from your agents in Lys.</p>\
<form method=\"post\" action=\"/oauth/mcp/consent\">\
<input type=\"hidden\" name=\"asking\" value=\"{asking}\">\
<fieldset><legend>How may this connection act?</legend>\
<label><input type=\"radio\" name=\"mode\" value=\"agent\" checked> Agent mode</label>\
<p>Use only the agent permissions chosen below.</p>\
<label><input type=\"radio\" name=\"mode\" value=\"person_reads\"> Person reads</label>\
<p>Read who you are, your own agents, your own directory drafts awaiting your decision \
or raised by you, and product drafts held by you or recorded with you as responsible, including waiting drafts. \
Each read is recorded with this connection, you and the route. \
It can never decide, approve, give or sign for you. Those acts need you in Lys in the browser. \
Leave agent permissions unticked when choosing Person reads.</p></fieldset>\
{choices}<p>Ticking nothing connects nothing in agent mode. The agent will have no permissions.</p>\
<button type=\"submit\" name=\"decision\" value=\"approve\">Connect {name}</button>\
<button type=\"submit\" name=\"decision\" value=\"refuse\">Don't connect</button>\
</form></body></html>"
        )),
    )
        .into_response())
}
