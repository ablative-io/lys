//! A snapshot keeps each product draft as its own events' canonical bytes
//! and positions, in the order they were recorded, and reads them back by
//! folding them again: the record read is the record that was written.

use super::{ProductDraftEvent, ProductDraftRecord};
use crate::product_draft_event;
use crate::projection::Projection;
use crate::state_value::{Unreadable, array, bytes, list, read_uint, tuple, uint};
use ciborium::Value;

pub(in crate::projection) fn encode(projection: &Projection) -> Value {
    array(
        projection
            .product_drafts
            .values()
            .map(|record| {
                array(
                    ProductDraftRecord::events(record)
                        .iter()
                        .map(|(event, index)| {
                            array(vec![
                                bytes(&product_draft_event::encode(event)),
                                uint(*index),
                            ])
                        })
                        .collect(),
                )
            })
            .collect(),
    )
}

pub(in crate::projection) fn decode(
    projection: &mut Projection,
    value: Value,
) -> Result<(), Unreadable> {
    for record in list(value, "the product draft records")? {
        let mut first = true;
        for held in list(record, "a product draft record")? {
            let [body, index] = tuple::<2>(held, "a product draft event")?;
            let Value::Bytes(body) = body else {
                return Err("a stored product draft event is canonical bytes".to_owned());
            };
            let event = product_draft_event::decode(&body).map_err(|error| error.to_string())?;
            let index = read_uint(&index, "a product draft event index")?;
            if first != matches!(event, ProductDraftEvent::Created(_)) {
                return Err("a product draft record starts with its creation alone".to_owned());
            }
            if projection.operation(event.operation()) != Some(index) {
                return Err("a product draft event disagrees with its operation index".to_owned());
            }
            if first && projection.product_draft(event.operation()).is_some() {
                return Err("a product draft is recorded twice".to_owned());
            }
            projection
                .fold_product_draft(&event, index)
                .map_err(|error| error.to_string())?;
            first = false;
        }
    }
    Ok(())
}
