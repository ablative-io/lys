use super::*;

const RUN: &str = "0123456789abcdef0123456789abcdef";

/// An upstream that answers every request with the path it was asked for.
async fn path_echo() -> Res<SocketAddr> {
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let addr = listener.local_addr()?;
    tokio::spawn(serve(listener, |request: Request<Incoming>| async move {
        let path = request
            .uri()
            .path_and_query()
            .map_or_else(String::new, |path| path.as_str().to_owned());
        match request.into_body().collect().await {
            Ok(_) => whole(StatusCode::OK, &json!({ "path": path })),
            Err(error) => whole(StatusCode::BAD_REQUEST, &json!({"fake": error.to_string()})),
        }
    }));
    Ok(addr)
}

/// A request as Codex sends one: its session in a header, and its `ChatGPT`
/// account's id when it is signed in with `ChatGPT`.
fn codex(method: &str, path: &str, chatgpt: bool) -> Res<Request<Full<Bytes>>> {
    let mut request = Request::builder()
        .method(method)
        .uri(path)
        .header(CONTENT_TYPE, "application/json")
        .header("session-id", "codex-session-1");
    if chatgpt {
        request = request.header("chatgpt-account-id", "account");
    }
    let body = if method == "POST" {
        Bytes::from(json!({"model": "gpt-test", "input": [], "stream": false}).to_string())
    } else {
        Bytes::new()
    };
    Ok(request.body(Full::new(body))?)
}

async fn asked(proxy: SocketAddr, request: Request<Full<Bytes>>) -> Res<Value> {
    let (response, _connection) = send(proxy, request).await?;
    Ok(serde_json::from_slice(
        &response.into_body().collect().await?.to_bytes(),
    )?)
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_codex_signed_in_with_chatgpt_is_answered_by_chatgpts_backend_and_linked_by_its_session()
-> Res {
    let harness = Harness::start(path_echo().await?).await?;
    let path = format!("/{RUN}/openai/v1/responses");
    // With an API key the call goes to the API as it came, without its key.
    let api = asked(harness.addr, codex("POST", &path, false)?).await?;
    assert_eq!(api["path"], "/v1/responses");
    let report = harness.report()?;
    assert_eq!(report.session, "codex-session-1");
    assert!(report.linked);
    // Signed in with ChatGPT, it goes to ChatGPT's backend, whose paths carry no /v1.
    let chatgpt = asked(harness.addr, codex("POST", &path, true)?).await?;
    assert_eq!(chatgpt["path"], "/chatgpt-backend/responses");
    let report = harness.report()?;
    assert_eq!(report.session, "codex-session-1");
    // What is not a model call follows the same route and is not recorded.
    let models = format!("/{RUN}/openai/v1/models?client_version=1");
    let listed = asked(harness.addr, codex("GET", &models, true)?).await?;
    assert_eq!(listed["path"], "/chatgpt-backend/models?client_version=1");
    let listed = asked(harness.addr, codex("GET", &models, false)?).await?;
    assert_eq!(listed["path"], "/v1/models?client_version=1");
    let calls = harness.calls("codex-session-1")?;
    assert_eq!(calls.len(), 2);
    for call in &calls {
        assert_eq!(call.api, Api::Responses);
        assert_eq!(call.provider, "openai");
        assert_eq!(call.run.as_deref(), Some(RUN));
    }
    Ok(())
}
