//! Restart fixtures and send requests through their live HTTP clients.

use std::error::Error;

use lys_identity_server::{Config, Say};

use super::{Answer, HARNESS_PASSWORD, Service, answer, location, session_cookie};
use crate::fake_issuer::Login;
use crate::harness_serve::{StageTimer, serve};

impl Service {
    /// Stop the service and start it again over the same directory, on a new
    /// address with a new client, so every store is opened from disk and no
    /// connection of the stopped service carries over. Signed-in sessions are
    /// among what is opened from disk: a cookie from before still answers.
    ///
    /// # Errors
    /// Propagates the service, transport or decoding failure for this fixture step.
    pub async fn restart(&mut self) -> Result<(), Box<dyn Error>> {
        Box::pin(self.restart_saying(None)).await
    }

    /// Restart as [`Service::restart`] does, saying each line of the
    /// restarted service's log to `say` when one is given.
    ///
    /// # Errors
    /// Propagates the service, transport or decoding failure for this fixture step.
    pub async fn restart_saying(&mut self, say: Option<Say>) -> Result<(), Box<dyn Error>> {
        Box::pin(self.restart_between(say, |_| Ok(()))).await
    }

    /// Restart as [`Service::restart_saying`] does, running `between` on the
    /// configuration once the service has stopped cleanly and written its
    /// stores receipt, and before it starts again: what a person or a tool
    /// does to the stores while Lys is stopped (DIRECTORY-107 R2).
    ///
    /// # Errors
    /// Propagates `between`'s failure and the service, transport or
    /// decoding failure for this fixture step.
    pub async fn restart_between(
        &mut self,
        say: Option<Say>,
        between: impl FnOnce(&Config) -> Result<(), Box<dyn Error>>,
    ) -> Result<(), Box<dyn Error>> {
        let timing = StageTimer::new("fixture.restart");
        self.server.stop()?;
        if let Some(at_stop) = self.at_stop.take() {
            at_stop.write()?;
        }
        between(&self.config)?;
        let stage = StageTimer::new("fixture.restart_rebind");
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
        let listen = listener.local_addr()?;
        self.base = format!("http://{listen}");
        self.config.listen = listen;
        self.config.redirect_url = format!("{}/callback", self.base);
        // The issuer sends a provider sign-in back to Lys's own address, which
        // moved with the listener.
        self.issuer
            .set_public_callback(&format!("{}/auth/v1/providers/callback", self.base))?;
        drop(stage);
        let (server, client, at_stop) = serve(listener, &self.config, say, &self.clock).await?;
        self.server = server;
        self.at_stop = at_stop;
        self.client = client;
        drop(timing);
        Ok(())
    }

    /// Restart an existing installation with the configuration an upgrade
    /// writes, retaining every durable log and credential.
    ///
    /// # Errors
    /// Propagates the service, transport or decoding failure for this fixture step.
    pub async fn restart_adjusted(
        &mut self,
        adjust: impl FnOnce(&mut Config),
    ) -> Result<(), Box<dyn Error>> {
        adjust(&mut self.config);
        self.restart().await
    }

    /// Begin a sign-in through a sign-in provider and let it answer as
    /// `login`, answering the callback path and the initiating browser's
    /// binding cookie. The browser must return that cookie on the callback.
    ///
    /// # Errors
    /// Propagates the service, transport or decoding failure for this fixture step.
    pub async fn issuer_answer(&self, login: Login) -> Result<(String, String), Box<dyn Error>> {
        self.issuer_answer_from(login, "harness").await
    }

    /// Complete the named provider's flight through the unmodified issuer protocol.
    ///
    /// # Errors
    /// Propagates fixture, transport and callback decoding failures.
    pub async fn issuer_answer_from(
        &self,
        login: Login,
        provider: &str,
    ) -> Result<(String, String), Box<dyn Error>> {
        let timing = StageTimer::new("http.issuer_answer");
        self.issuer.sign_in_as(login)?;
        let to_issuer = self
            .client
            .get(format!("{}/sign-in/providers/{provider}", self.base))
            .send()
            .await?;
        let authorize = location(&to_issuer)?;
        let binding = to_issuer
            .headers()
            .get(reqwest::header::SET_COOKIE)
            .ok_or_else(|| {
                format!(
                    "the provider start did not bind the browser: {} to {authorize}",
                    to_issuer.status()
                )
            })?
            .to_str()?
            .split(';')
            .next()
            .ok_or("the provider start set no cookie value")?
            .to_owned();
        let back = self.client.get(authorize).send().await?;
        let url = location(&back)?;
        let path = url
            .strip_prefix(&self.base)
            .ok_or_else(|| format!("the issuer sent the browser to {url}"))?
            .to_owned();
        drop(timing);
        Ok((path, binding))
    }

    /// Sign in on Lys's own sign-in route as `login`, which the issuer
    /// signs whatever password is typed, answering the session cookie.
    ///
    /// # Errors
    /// Propagates the service, transport or decoding failure for this fixture step.
    pub async fn sign_in(&self, login: Login) -> Result<String, Box<dyn Error>> {
        let timing = StageTimer::new("http.sign_in");
        let email = login.email.clone();
        self.issuer.sign_in_as(login)?;
        let answer = self.sign_in_with(&email, HARNESS_PASSWORD).await;
        drop(timing);
        answer
    }

    /// Sign in on Lys's own sign-in route with `email` and `password`,
    /// answering the session cookie, or the refusal as an error.
    ///
    /// # Errors
    /// Propagates the service, transport or decoding failure for this fixture step.
    pub async fn sign_in_with(
        &self,
        email: &str,
        password: &str,
    ) -> Result<String, Box<dyn Error>> {
        let timing = StageTimer::new("http.sign_in_request");
        let body = serde_json::json!({ "email": email, "password": password });
        let signed_in = self
            .client
            .post(format!("{}/sign-in", self.base))
            .header(reqwest::header::CONTENT_TYPE, "application/json")
            .body(body.to_string())
            .send()
            .await?;
        let answer = session_cookie(signed_in).await;
        drop(timing);
        answer
    }

    /// GET `path`, with the session `cookie` when one is given.
    ///
    /// # Errors
    /// Propagates the service, transport or decoding failure for this fixture step.
    pub async fn get(&self, path: &str, cookie: Option<&str>) -> Result<Answer, Box<dyn Error>> {
        let timing = StageTimer::new("http.get");
        let mut request = self.client.get(format!("{}{path}", self.base));
        if let Some(cookie) = cookie {
            request = request.header(reqwest::header::COOKIE, cookie);
        }
        let answer = answer(request.send().await?).await;
        drop(timing);
        answer
    }

    /// GET `path` with the initiating browser's cookie, asking for a page: the
    /// status, the redirect it names, and whether it set a session cookie.
    ///
    /// # Errors
    /// Propagates the service, transport or decoding failure for this fixture step.
    pub async fn get_page(
        &self,
        path: &str,
        binding: &str,
    ) -> Result<(u16, Option<String>, bool), Box<dyn Error>> {
        let timing = StageTimer::new("http.get_page");
        let response = self
            .client
            .get(format!("{}{path}", self.base))
            .header(reqwest::header::ACCEPT, "text/html,application/xhtml+xml")
            .header(reqwest::header::COOKIE, binding)
            .send()
            .await?;
        let status = response.status().as_u16();
        let to = location(&response).ok();
        let cookie = response.headers().contains_key(reqwest::header::SET_COOKIE);
        drop(timing);
        Ok((status, to, cookie))
    }

    /// POST `body` as JSON to `path`, with the session `cookie` when one is given.
    ///
    /// # Errors
    /// Propagates the service, transport or decoding failure for this fixture step.
    pub async fn post(
        &self,
        path: &str,
        cookie: Option<&str>,
        body: &serde_json::Value,
    ) -> Result<Answer, Box<dyn Error>> {
        let timing = StageTimer::new("http.post");
        let mut request = self
            .client
            .post(format!("{}{path}", self.base))
            .header(reqwest::header::CONTENT_TYPE, "application/json")
            .body(body.to_string());
        if let Some(cookie) = cookie {
            request = request.header(reqwest::header::COOKIE, cookie);
        }
        let answer = answer(request.send().await?).await;
        drop(timing);
        answer
    }

    /// The directory log's size as the public receipt route's checkpoint
    /// names it, so a test can prove a refused call logged nothing without
    /// a session. The checkpoint counts every leaf, an install event's too.
    ///
    /// # Errors
    /// Propagates the service, transport or decoding failure for this fixture step.
    pub async fn log_size(&self) -> Result<u64, Box<dyn Error>> {
        let timing = StageTimer::new("http.log_size");
        let (status, first) = self.get("/receipts/0", None).await?;
        if status != 200 {
            return Ok(0);
        }
        let size = first["checkpoint"]["tree_size"]
            .as_u64()
            .ok_or_else(|| format!("the first receipt names no tree size: {first}").into());
        drop(timing);
        size
    }

    /// POST the JSON `body` bytes to `path` carrying the header `name: value`
    /// and no session cookie, so a request signed over those exact bytes
    /// arrives as signed.
    ///
    /// # Errors
    /// Propagates the service, transport or decoding failure for this fixture step.
    pub async fn post_signed(
        &self,
        path: &str,
        (name, value): (&str, &str),
        body: Vec<u8>,
    ) -> Result<Answer, Box<dyn Error>> {
        let timing = StageTimer::new("http.post_signed");
        let request = self
            .client
            .post(format!("{}{path}", self.base))
            .header(reqwest::header::CONTENT_TYPE, "application/json")
            .header(name, value)
            .body(body);
        let answer = answer(request.send().await?).await;
        drop(timing);
        answer
    }

    /// POST the JSON `body` bytes to `path` carrying every header of
    /// `headers`, so a request may carry a signature over those exact bytes
    /// and a session cookie at once.
    ///
    /// # Errors
    /// Propagates the service, transport or decoding failure for this fixture step.
    pub async fn post_carrying(
        &self,
        path: &str,
        headers: &[(&str, &str)],
        body: Vec<u8>,
    ) -> Result<Answer, Box<dyn Error>> {
        let timing = StageTimer::new("http.post_carrying");
        let mut request = self
            .client
            .post(format!("{}{path}", self.base))
            .header(reqwest::header::CONTENT_TYPE, "application/json")
            .body(body);
        for (name, value) in headers {
            request = request.header(*name, *value);
        }
        let answer = answer(request.send().await?).await;
        drop(timing);
        answer
    }
}
