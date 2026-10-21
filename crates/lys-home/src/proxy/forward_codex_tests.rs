#![cfg(test)]
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

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_codex_client_closing_after_response_completed_keeps_completion_and_usage() -> Res {
    let (finish_upstream, completion) = tokio::sync::oneshot::channel::<()>();
    let completion = Arc::new(std::sync::Mutex::new(Some(completion)));
    let (upstream, _) = fake(move || {
        let completion = Arc::clone(&completion);
        async move {
            let (tx, response) = fed(StatusCode::OK, "text/event-stream");
            tokio::spawn(async move {
                let event = json!({"type": "response.completed", "response": {
                    "id": "response-final", "status": "completed",
                    "output": [{"type": "message", "role": "assistant", "content": [{"type": "output_text", "text": "OK"}]}],
                    "usage": {"input_tokens": 31, "output_tokens": 7, "input_tokens_details": {"cached_tokens": 11}}
                }});
                if tx.send(Bytes::from(format!("event: response.completed\ndata: {event}\n\n"))).await.is_err() {
                    return;
                }
                let receiver = completion.lock().expect("fixture lock").take();
                if let Some(receiver) = receiver
                    && let Err(error) = receiver.await {
                    eprintln!("upstream hold ended: {error}");
                }
                drop(tx);
            });
            response
        }
    }).await?;
    let harness = Harness::start(upstream).await?;
    let path = format!("/{RUN}/openai/v1/responses");
    let mut request = codex("POST", &path, true)?;
    *request.body_mut() = Full::new(Bytes::from(
        json!({"model": "gpt-test", "input": [], "stream": true}).to_string(),
    ));
    let (response, connection) = send(harness.addr, request).await?;
    let mut body = response.into_body();
    let mut received = Vec::new();
    while !received.ends_with(b"\n\n") {
        let frame = body.frame().await.ok_or("no completed event")??;
        if let Some(data) = frame.data_ref() {
            received.extend_from_slice(data);
        }
    }
    drop(body);
    connection.abort();
    let report = harness.report()?;
    assert_eq!(report.status, CallStatus::Complete);
    let calls = harness.calls("codex-session-1")?;
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].status, CallStatus::Complete);
    assert_eq!(calls[0].response.len(), 1);
    let usage = calls[0].usage.as_ref().ok_or("no recorded usage")?;
    assert_eq!(usage.input, Some(31));
    assert_eq!(usage.output, Some(7));
    assert_eq!(usage.cache_read, Some(11));
    finish_upstream
        .send(())
        .map_err(|()| "upstream hold disappeared")?;
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn an_upstream_429_is_recorded_as_refused_with_its_status() -> Res {
    let (upstream, _) = fake(|| async {
        whole(
            StatusCode::TOO_MANY_REQUESTS,
            &json!({"error": {"message": "rate limited"}}),
        )
    })
    .await?;
    let harness = Harness::start(upstream).await?;
    let path = format!("/{RUN}/openai/v1/responses");
    let (response, _connection) = send(harness.addr, codex("POST", &path, false)?).await?;
    assert_eq!(response.status(), StatusCode::TOO_MANY_REQUESTS);
    response.into_body().collect().await?;
    harness.report()?;
    let calls = harness.calls("codex-session-1")?;
    assert_eq!(calls.len(), 1);
    assert_eq!(serde_json::to_value(calls[0].status)?, "upstream_refused");
    assert_eq!(
        calls[0].head.as_ref().ok_or("no upstream head")?.status,
        Some(429)
    );
    assert!(calls[0].response.is_empty());
    assert!(calls[0].unrecorded_reason.is_none());
    Ok(())
}
