//! Bind one consent choice to one resource and its complete set of offered acts.

use std::collections::{BTreeMap, BTreeSet};

use sha2::{Digest, Sha256};

use super::Offer;
use crate::error::ServerError;

pub(crate) struct Choice {
    pub(crate) key: String,
    kind: String,
    id: String,
    actions: BTreeSet<String>,
    offers: Vec<Offer>,
}

impl Choice {
    pub(crate) fn words(&self) -> String {
        super::words(
            &self.kind,
            &self.id,
            &self.actions.iter().cloned().collect::<Vec<_>>(),
        )
    }

    pub(crate) fn pick(mut self) -> Result<Vec<Offer>, ServerError> {
        let mut picked = Vec::new();
        while !self.actions.is_empty() {
            let next = self
                .offers
                .iter()
                .enumerate()
                .map(|(at, offer)| {
                    (
                        at,
                        offer
                            .actions
                            .iter()
                            .filter(|action| self.actions.contains(*action))
                            .count(),
                    )
                })
                .max_by_key(|(_, count)| *count);
            let Some((at, _)) = next.filter(|(_, count)| *count > 0) else {
                return Err(ServerError::RequestMalformed {
                    reason: "the chosen resource's offered acts no longer have a source".to_owned(),
                });
            };
            let offer = self.offers.swap_remove(at);
            for action in &offer.actions {
                self.actions.remove(action);
            }
            picked.push(offer);
        }
        Ok(picked)
    }
}

pub(crate) fn grouped(offers: Vec<Offer>) -> Result<Vec<Choice>, ServerError> {
    let mut resources: BTreeMap<(String, String), Vec<Offer>> = BTreeMap::new();
    for offer in offers {
        resources
            .entry((offer.kind.clone(), offer.id.clone()))
            .or_default()
            .push(offer);
    }
    resources
        .into_iter()
        .map(|((kind, id), mut offers)| {
            offers.sort_by(|a, b| a.key.cmp(&b.key));
            let actions: BTreeSet<String> = offers
                .iter()
                .flat_map(|offer| offer.actions.iter().cloned())
                .collect();
            let keys: Vec<_> = offers.iter().map(|offer| &offer.key).collect();
            let binding = serde_json::to_vec(&(&kind, &id, &actions, keys)).map_err(|error| {
                ServerError::RequestMalformed {
                    reason: format!("the consent choices could not be bound: {error}"),
                }
            })?;
            let key = format!("resource:{}", crate::routes::hex(&Sha256::digest(binding)));
            Ok(Choice {
                key,
                kind,
                id,
                actions,
                offers,
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{Offer, grouped};
    use std::error::Error;

    fn offer(key: &str, actions: &[&str]) -> Offer {
        Offer {
            key: key.to_owned(),
            source: key.to_owned(),
            kind: "project".to_owned(),
            id: "ward".to_owned(),
            relation: "holder".to_owned(),
            actions: actions.iter().map(|action| (*action).to_owned()).collect(),
        }
    }

    #[test]
    fn a_resource_selects_only_the_sources_needed_to_cover_its_displayed_acts()
    -> Result<(), Box<dyn Error>> {
        let mut groups = grouped(vec![
            offer("reader", &["read"]),
            offer("editor", &["read", "write"]),
            offer("another-editor", &["read", "write"]),
        ])?;
        assert_eq!(groups.len(), 1);
        let picked = groups.remove(0).pick()?;
        assert_eq!(picked.len(), 1);
        assert_eq!(picked[0].actions, ["read", "write"]);
        Ok(())
    }

    #[test]
    fn a_choice_is_bound_to_raw_acts_even_when_their_display_words_match()
    -> Result<(), Box<dyn Error>> {
        let before = grouped(vec![offer("source:member", &["network.publish"])])?;
        let after = grouped(vec![offer("source:member", &["network_publish"])])?;
        assert_eq!(before[0].words(), after[0].words());
        assert_ne!(before[0].key, after[0].key);
        Ok(())
    }
}
