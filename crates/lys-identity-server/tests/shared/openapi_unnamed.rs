//! The routes the `OpenAPI` document names no answer or no body for, with
//! why, which `tests/openapi.rs` holds exact from both sides.

/// Every route whose answer the document describes by its words alone, with
/// why. `every_route_names_the_types_it_takes_and_answers` holds this list
/// exact from both sides: a route named here that in fact names an answer
/// fails, and a route that names none and is not here fails.
pub(crate) const OPEN_ANSWERS: &[(&str, &str)] = &[
    (
        "get /.well-known/openid-configuration",
        "the discovery document follows OpenID Connect, not a Lys type",
    ),
    (
        "get /oauth/authorize",
        "answers a redirect to the client or the sign-in page, with no body",
    ),
    (
        "post /oauth/token",
        "answers the OAuth token response, which follows RFC 6749, not a Lys type",
    ),
    (
        "get /oauth/jwks",
        "answers a JSON Web Key Set, which follows RFC 7517, not a Lys type",
    ),
    (
        "get /oauth/userinfo",
        "answers the OpenID Connect claims of the bearer's subject",
    ),
    (
        "get /.well-known/oauth-protected-resource",
        "answers the protected resource document, which follows RFC 9728, not a Lys type",
    ),
    (
        "get /.well-known/oauth-protected-resource/mcp",
        "answers the protected resource document, which follows RFC 9728, not a Lys type",
    ),
    (
        "get /.well-known/oauth-protected-resource/api/mcp",
        "answers the protected resource document, which follows RFC 9728, not a Lys type",
    ),
    (
        "get /.well-known/oauth-authorization-server",
        "answers the authorization server document, which follows RFC 8414, not a Lys type",
    ),
    (
        "post /oauth/mcp/register",
        "answers the client registration, which follows RFC 7591, not a Lys type",
    ),
    (
        "get /oauth/mcp/authorize",
        "answers the approval page as HTML, or a redirect back to the app",
    ),
    (
        "post /oauth/mcp/consent",
        "answers a redirect back to the app, with no body",
    ),
    (
        "post /oauth/mcp/token",
        "answers the OAuth token response, which follows RFC 6749, not a Lys type",
    ),
    (
        "post /sign-in",
        "answers through Response, since it must set the session cookie",
    ),
    (
        "get /sign-in/providers",
        "answers the offered providers as a JSON list built in place",
    ),
    (
        "get /sign-in/providers/{id}",
        "answers a redirect to the provider, with no body",
    ),
    (
        "post /setup/open",
        "answers the setup state as a JSON object built in place",
    ),
    (
        "post /setup/administrator",
        "answers through Response, since it must set the session cookie",
    ),
    (
        "post /setup/password",
        "answers through Response, since it must set the session cookie",
    ),
    (
        "get /me/account",
        "answers the caller's account as the issuer reports it",
    ),
    (
        "post /me/account/email",
        "answers the account as the issuer reports it after the change",
    ),
    (
        "post /me/account/password",
        "answers the account as the issuer reports it after the change",
    ),
    (
        "get /directory/people/{id}/account",
        "answers the person's account as the issuer reports it",
    ),
    (
        "post /directory/people/{id}/account/email",
        "answers the account as the issuer reports it after the change",
    ),
    (
        "post /directory/people/{id}/account/enabled",
        "answers the account as the issuer reports it after the change",
    ),
    (
        "post /directory/people/{id}/account/password",
        "answers the account as the issuer reports it after the change",
    ),
    ("get /authority", "answers text/plain, not JSON"),
    ("get /login", "answers a 303 to /#/sign-in, with no body"),
    (
        "get /callback",
        "answers through Response, since it must set the session cookie",
    ),
    (
        "get /configuration",
        "answers the startup settings as a dump that follows the configuration",
    ),
    ("get /secrets", "the secrets broker's own answer, forwarded"),
    (
        "get /secrets/grants",
        "the secrets broker's own answer, forwarded",
    ),
    (
        "get /secrets/audit",
        "the secrets broker's own answer, forwarded",
    ),
    (
        "get /secrets/revocation",
        "the secrets broker's own answer, forwarded",
    ),
    (
        "get /secrets/settings",
        "the secrets broker's own answer, forwarded",
    ),
    (
        "get /secrets/handles",
        "the secrets broker's own answer, forwarded",
    ),
    (
        "post /secrets/scope",
        "the secrets broker's own answer, forwarded",
    ),
    (
        "post /secrets/recipients",
        "the secrets broker's own answer, forwarded",
    ),
    (
        "post /secrets/drop",
        "the secrets broker's own answer, forwarded",
    ),
    (
        "post /agents/{id}/start",
        "the start owners' own rendered JSON, which this crate holds no type for",
    ),
    (
        "post /launch-records/{id}/start-again",
        "the start owners' own rendered JSON, which this crate holds no type for",
    ),
    (
        "post /launch-records/{id}/withdraw",
        "the start owners' own rendered JSON, which this crate holds no type for",
    ),
    (
        "get /launch-records/{id}/state",
        "the start owners' own rendered JSON, which this crate holds no type for",
    ),
    ("get /openapi.json", "answers this document"),
    (
        "post /apps/{app}/placements",
        "answers the placement made, with nothing of its own to say",
    ),
    ("post /apps/bench", "answers the bench opened"),
    ("post /apps/bench/{id}/close", "answers the bench closed"),
    (
        "post /runtime/sessions/{id}/input",
        "answers the runner's own answer and the act's receipt",
    ),
    (
        "post /runtime/sessions/{id}/input-bytes",
        "answers the runner's own answer and the act's receipt",
    ),
    (
        "post /runtime/sessions/{id}/read-bytes",
        "answers the runner's own answer and the act's receipt",
    ),
    (
        "post /runtime/sessions/{id}/keys",
        "answers the runner's own answer and the act's receipt",
    ),
    (
        "post /runtime/sessions/{id}/read",
        "answers the runner's own answer and the act's receipt",
    ),
    (
        "post /runtime/sessions/{id}/wait",
        "answers the runner's own answer and the act's receipt",
    ),
    (
        "post /runtime/sessions/{id}/resize",
        "answers the runner's own answer and the act's receipt",
    ),
    (
        "post /runtime/sessions/{id}/compact",
        "answers the runner's own answer and the act's receipt",
    ),
    (
        "post /runtime/sessions/{id}/end",
        "answers the runner's own answer and the act's receipt",
    ),
    (
        "post /agents/{id}/wake",
        "answers the runner's own answer and the act's receipt",
    ),
    (
        "get /runtime/live",
        "answers the live sessions beside the runners that did not answer",
    ),
    (
        "get /network/machines/{id}/runner",
        "answers the machine and its runner record",
    ),
    (
        "post /network/machines/{id}/runner",
        "answers the machine and its runner record",
    ),
    (
        "get /runner/protocol",
        "answers the runner protocol's own published section",
    ),
    (
        "post /runner/dial/{machine}/next",
        "answers a signed line of the runner protocol, not JSON",
    ),
    (
        "post /runner/dial/{machine}/replies/{ticket}",
        "answers the ticket delivered, with nothing of its own to say",
    ),
];

/// Every POST or PUT route the document gives no request body schema, with
/// why. Each either takes no body at all or takes one it does not own.
pub(crate) const NO_BODY: &[(&str, &str)] = &[
    (
        "post /oauth/token",
        "takes a form-encoded body, as RFC 6749 requires",
    ),
    (
        "post /oauth/mcp/token",
        "takes a form-encoded body, as RFC 6749 requires",
    ),
    (
        "post /oauth/mcp/register",
        "takes the client metadata RFC 7591 defines, not a Lys type",
    ),
    (
        "post /oauth/mcp/consent",
        "takes the approval page's form-encoded answer",
    ),
    ("post /requests/{id}/reconcile", "takes no body"),
    ("post /grants/agent-roots", "takes no body"),
    ("post /network/machines/{id}/retire", "takes no body"),
    ("post /sessions/{id}/end", "takes no body"),
    (
        "post /grants/{id}/tokens/{token_id}/revoke",
        "takes no body: the token is named in the path",
    ),
    (
        "post /directory/people/{id}/sessions/{session}/end",
        "takes no body",
    ),
    ("post /launch-records/{id}/start-again", "takes no body"),
    ("post /launch-records/{id}/withdraw", "takes no body"),
    ("post /apps/bench/{id}/close", "takes no body"),
    (
        "post /secrets/scope",
        "forwards its bytes to the secrets broker unread",
    ),
    (
        "post /secrets/recipients",
        "forwards its bytes to the secrets broker unread",
    ),
    (
        "post /secrets/drop",
        "forwards its bytes to the secrets broker unread",
    ),
    (
        "post /agents/{id}/start",
        "takes the start grammar's members as they came, not a Rust type",
    ),
    (
        "post /runner/dial/{machine}/next",
        "takes the runner's greeting line, signed by the machine's key",
    ),
    (
        "post /runner/dial/{machine}/replies/{ticket}",
        "takes the runner's reply line, signed by the machine's key",
    ),
];
