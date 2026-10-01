//! Derived locations retain the fold's app, history, placement and registrar order.

use std::collections::HashMap;

use super::{App, Placed, Registrar};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Location {
    Registered(usize),
    History(usize, usize),
    Placed(usize),
    Registrar(usize),
}

impl Location {
    fn order(self) -> (usize, usize, usize) {
        match self {
            Self::Registered(app) => (0, app, 0),
            Self::History(app, history) => (0, app, history + 1),
            Self::Placed(placed) => (1, placed, 0),
            Self::Registrar(registrar) => (2, registrar, 0),
        }
    }
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub(super) struct Index {
    pub(super) apps: HashMap<String, usize>,
    pub(super) operations: HashMap<String, Location>,
    parents: HashMap<String, HashMap<String, usize>>,
}

impl Index {
    pub(super) fn of(apps: &[App], placements: &[Placed], registrars: &[Registrar]) -> Self {
        let mut index = Self::default();
        for (position, app) in apps.iter().enumerate() {
            index.app(&app.registered.app, position);
            index.operation(&app.registered.operation, Location::Registered(position));
            for (history, line) in app.history.iter().enumerate() {
                #[cfg(test)]
                super::operation_tests::visited();
                index.operation(line.operation(), Location::History(position, history));
            }
        }
        for (position, placed) in placements.iter().enumerate() {
            index.placement(placed, position);
            index.operation(&placed.operation, Location::Placed(position));
        }
        for (position, registrar) in registrars.iter().enumerate() {
            index.operation(&registrar.operation, Location::Registrar(position));
        }
        index
    }

    pub(super) fn app(&mut self, id: &str, position: usize) {
        self.apps.entry(id.to_owned()).or_insert(position);
    }

    pub(super) fn operation(&mut self, operation: &str, location: Location) {
        self.operations
            .entry(operation.to_owned())
            .and_modify(|previous| {
                if location.order() < previous.order() {
                    *previous = location;
                }
            })
            .or_insert(location);
    }

    pub(super) fn placement(&mut self, placed: &Placed, position: usize) {
        self.parents
            .entry(placed.child_kind.clone())
            .or_default()
            .entry(placed.child_id.clone())
            .or_insert(position);
    }

    pub(super) fn parent(&self, kind: &str, id: &str) -> Option<usize> {
        self.parents.get(kind)?.get(id).copied()
    }
}
