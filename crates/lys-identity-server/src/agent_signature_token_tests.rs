//! While a call admitted by a grant token is served, its holder is held to
//! that token's grant alone, and no other caller is.

use lys_identity::grants::GrantId;
use lys_identity::{AgentId, IdentityId, PersonId};

use super::{TokenPrincipal, token_grant, token_scoped};

#[tokio::test]
async fn a_token_call_holds_only_its_holder_to_its_own_grant() {
    let holder = AgentId::from_bytes([1; 16]);
    let other = AgentId::from_bytes([2; 16]);
    let grant = GrantId::from_bytes([3; 16]);
    assert_eq!(token_grant(IdentityId::Agent(holder)), None);
    let principal = TokenPrincipal { holder, grant };
    let seen = token_scoped(principal, async move {
        (
            token_grant(IdentityId::Agent(holder)),
            token_grant(IdentityId::Agent(other)),
            token_grant(IdentityId::Person(PersonId::from_bytes([4; 16]))),
        )
    })
    .await;
    assert_eq!(seen, (Some(grant), None, None));
    assert_eq!(token_grant(IdentityId::Agent(holder)), None);
}
