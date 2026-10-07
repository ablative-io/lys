//! A handshake binds the transport without sending a turn.

use serde_json::json;

use super::{Controller, Dispatch, Transport, Update};

impl Controller {
    /// Handshake frames only; process launch and adapter qualification remain separate.
    #[must_use]
    pub fn bootstrap(&self) -> Update {
        if self.transport == Transport::Claude {
            return Update {
                dispatches: vec![Dispatch {
                    operation: String::new(),
                    frame: json!({"type":"control_request","request_id":self.initialize,
                        "request":{"subtype":"initialize"}}),
                }],
                ..Update::default()
            };
        }
        if self.transport != Transport::Codex {
            return Update::default();
        }
        Update {
            dispatches: vec![Dispatch {
                operation: String::new(),
                frame: json!({
                    "id":"lys-initialize", "method":"initialize", "params":{
                        "clientInfo":{"name":"lys","title":null,"version":env!("CARGO_PKG_VERSION")},
                        "capabilities":{"experimentalApi":false}
                    }
                }),
            }],
            ..Update::default()
        }
    }
}
