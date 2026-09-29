#![cfg(test)]
//! Historical replay never acts as fresh consent or issues another credential.

use crate::delegation::fixture::{TestResult, current, owner, request};

use super::*;

const APPROVAL: &str = "fixture-approval-nonce";
const REVOCATION: &str = "fixture-revoke-nonce";

fn approved(request: &Request) -> TestResult<(DecisionCommand, DecisionRecord)> {
    let command = DecisionCommand::new(
        "decide-1".to_owned(),
        request.clone(),
        Choice::Approve,
        APPROVAL,
    )?;
    let nonce = ActionNonce::for_decision(request.clone(), Choice::Approve, APPROVAL, 90)?;
    let DecisionPlan::Append(record) =
        plan_decision(&command, None, Some(&nonce), &current(request))?
    else {
        return Err("initial decision was replayed".into());
    };
    Ok((command, record))
}

#[test]
fn exact_retry_returns_original_receipt_after_nonce_consumption() -> TestResult {
    let request = request()?;
    let (command, record) = approved(&request)?;
    assert_eq!(
        plan_decision(&command, Some(&record), None, &current(&request))?,
        DecisionPlan::Replay(record.clone())
    );
    assert_eq!(standing(&record, false, &current(&request)), Ok(()));
    assert_eq!(
        plan_decision(&command, None, None, &current(&request)),
        Err(Refusal::NonceRefused)
    );
    Ok(())
}

#[test]
fn changed_body_or_proof_under_same_operation_is_refused() -> TestResult {
    let request = request()?;
    let (command, record) = approved(&request)?;
    let mut variants = Vec::new();
    let mut operations = command.clone();
    operations.request.operations.insert(Operation::Grants);
    variants.push(operations);
    let mut choice = command.clone();
    choice.choice = Choice::Decline;
    variants.push(choice);
    let mut expiry = command.clone();
    expiry.request.expires_at = 99;
    variants.push(expiry);
    let mut nonce = command;
    nonce.proof = [7; 32];
    variants.push(nonce);
    for changed in variants {
        assert_eq!(
            plan_decision(&changed, Some(&record), None, &current(&request)),
            Err(Refusal::OperationReused)
        );
    }
    Ok(())
}

#[test]
fn replay_after_withdrawal_or_binding_change_is_historical_only() -> TestResult {
    let request = request()?;
    let (command, record) = approved(&request)?;
    let mut facts = current(&request);
    facts.binding = None;
    assert_eq!(
        plan_decision(&command, Some(&record), None, &facts)?,
        DecisionPlan::Replay(record.clone())
    );
    assert_eq!(
        standing(&record, false, &facts),
        Err(Refusal::BindingChanged)
    );
    assert_eq!(
        standing(&record, true, &current(&request)),
        Err(Refusal::NotStanding)
    );
    assert_eq!(
        plan_decision(&command, Some(&record), None, &current(&request))?,
        DecisionPlan::Replay(record)
    );
    Ok(())
}

#[test]
fn original_owner_session_and_active_lifetime_are_required_even_for_replay() -> TestResult {
    let request = request()?;
    let (command, record) = approved(&request)?;
    let other = owner(2);
    let mut facts = current(&request);
    facts.owner = &other;
    assert_eq!(
        plan_decision(&command, Some(&record), None, &facts),
        Err(Refusal::OwnerRefused)
    );
    let mut facts = current(&request);
    facts.active = false;
    assert_eq!(
        plan_decision(&command, Some(&record), None, &facts),
        Err(Refusal::OwnerRefused)
    );
    let mut facts = current(&request);
    facts.at = 100;
    assert_eq!(
        plan_decision(&command, Some(&record), None, &facts),
        Err(Refusal::Expired)
    );
    Ok(())
}

#[test]
fn revoke_requires_its_own_action_nonce_bound_to_current_owner_session() -> TestResult {
    let request = request()?;
    let (_, decision) = approved(&request)?;
    let command = RevocationCommand::new(
        "revoke-1".to_owned(),
        request.consent.clone(),
        request.owner.clone(),
        APPROVAL,
    )?;
    let approval = ActionNonce::for_decision(request.clone(), Choice::Approve, APPROVAL, 90)?;
    assert_eq!(
        plan_revocation(
            &command,
            &decision,
            None,
            Some(&approval),
            &current(&request)
        ),
        Err(Refusal::NonceRefused)
    );
    let mut later = request.owner.clone();
    later.session = "a".repeat(32);
    let command = RevocationCommand::new(
        "revoke-1".to_owned(),
        request.consent.clone(),
        later.clone(),
        REVOCATION,
    )?;
    let nonce =
        ActionNonce::for_revocation(later.clone(), request.consent.clone(), REVOCATION, 90)?;
    let mut facts = current(&request);
    facts.owner = &later;
    facts.binding = None;
    let RevocationPlan::Append(record) =
        plan_revocation(&command, &decision, None, Some(&nonce), &facts)?
    else {
        return Err("first withdrawal was replayed".into());
    };
    assert_eq!(
        plan_revocation(&command, &decision, Some(&record), None, &facts)?,
        RevocationPlan::Replay(record)
    );
    assert_eq!(standing(&decision, true, &facts), Err(Refusal::NotStanding));
    Ok(())
}

#[test]
fn nonce_cannot_be_retargeted_to_another_request_choice_or_revoke_session() -> TestResult {
    let request = request()?;
    let nonce = ActionNonce::for_decision(request.clone(), Choice::Approve, APPROVAL, 90)?;
    let command = DecisionCommand::new(
        "decide-1".to_owned(),
        request.clone(),
        Choice::Decline,
        APPROVAL,
    )?;
    assert_eq!(
        plan_decision(&command, None, Some(&nonce), &current(&request)),
        Err(Refusal::NonceRefused)
    );
    let mut other = request.clone();
    other.consent = "consent-2".to_owned();
    let command = DecisionCommand::new("decide-1".to_owned(), other, Choice::Approve, APPROVAL)?;
    assert_eq!(
        plan_decision(&command, None, Some(&nonce), &current(&request)),
        Err(Refusal::NonceRefused)
    );
    let (_, record) = approved(&request)?;
    let stale = ActionNonce::for_revocation(owner(2), request.consent.clone(), REVOCATION, 90)?;
    let revoke = RevocationCommand::new(
        "revoke-1".to_owned(),
        request.consent.clone(),
        request.owner.clone(),
        REVOCATION,
    )?;
    assert_eq!(
        plan_revocation(&revoke, &record, None, Some(&stale), &current(&request)),
        Err(Refusal::NonceRefused)
    );
    Ok(())
}

#[test]
fn missing_pending_nonce_after_restart_cannot_reissue_a_decision() -> TestResult {
    let request = request()?;
    let command = DecisionCommand::new(
        "decide-1".to_owned(),
        request.clone(),
        Choice::Approve,
        APPROVAL,
    )?;
    assert_eq!(
        plan_decision(&command, None, None, &current(&request)),
        Err(Refusal::NonceRefused)
    );
    // There is deliberately no code/token field or exchange retry function.
    Ok(())
}

#[test]
fn new_decision_refuses_changed_binding_and_expired_nonce() -> TestResult {
    let request = request()?;
    let command = DecisionCommand::new(
        "decide-1".to_owned(),
        request.clone(),
        Choice::Approve,
        APPROVAL,
    )?;
    let nonce = ActionNonce::for_decision(request.clone(), Choice::Approve, APPROVAL, 30)?;
    let mut facts = current(&request);
    facts.binding = None;
    assert_eq!(
        plan_decision(&command, None, Some(&nonce), &facts),
        Err(Refusal::BindingChanged)
    );
    let mut facts = current(&request);
    facts.at = 30;
    assert_eq!(
        plan_decision(&command, None, Some(&nonce), &facts),
        Err(Refusal::Expired)
    );
    Ok(())
}

#[test]
fn changed_revocation_retry_never_appends_again() -> TestResult {
    let request = request()?;
    let (_, decision) = approved(&request)?;
    let command = RevocationCommand::new(
        "revoke-1".to_owned(),
        request.consent.clone(),
        request.owner.clone(),
        REVOCATION,
    )?;
    let nonce = ActionNonce::for_revocation(
        request.owner.clone(),
        request.consent.clone(),
        REVOCATION,
        90,
    )?;
    let RevocationPlan::Append(record) =
        plan_revocation(&command, &decision, None, Some(&nonce), &current(&request))?
    else {
        return Err("first withdrawal was replayed".into());
    };
    let mut changed = command;
    changed.proof = [8; 32];
    assert_eq!(
        plan_revocation(&changed, &decision, Some(&record), None, &current(&request)),
        Err(Refusal::OperationReused)
    );
    Ok(())
}
