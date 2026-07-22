use crate::event::payloads::{
    OnMetadataLocalTreeReadyEventCond, OnMetadataRawTreeReadyEventCond, OnServerErrorEventCond,
};

/// Defines how a registration cond decides whether a dispatch cond qualifies.
///
/// No default implementation because matching semantics differ per type:
/// `Vec<String>` fields use full containment (all dispatch values must be
/// within the registration set); `String` fields use exact equality.
/// Using `PartialEq` on Vec fields is wrong — a plugin may register multiple
/// values while a dispatch carries only a subset.
pub trait EventCond {
    fn matches(&self, dispatch_cond: &Self) -> bool;
}

impl EventCond for OnMetadataRawTreeReadyEventCond {
    fn matches(&self, dispatch_cond: &Self) -> bool {
        dispatch_cond.r#type.iter().all(|t| self.r#type.contains(t))
    }
}

impl EventCond for OnMetadataLocalTreeReadyEventCond {
    /// Only `r#type` participates in matching. `exclude_from_plugins` is
    /// metadata for the wasm handler, not a routing criterion.
    fn matches(&self, dispatch_cond: &Self) -> bool {
        dispatch_cond.r#type.iter().all(|t| self.r#type.contains(t))
    }
}

impl EventCond for OnServerErrorEventCond {
    fn matches(&self, dispatch_cond: &Self) -> bool {
        self.level == dispatch_cond.level
    }
}

impl EventCond for () {
    fn matches(&self, _: &Self) -> bool {
        true
    }
}

/// A type-erased cond container used by the host for storage and routing.
///
/// Each variant wraps a concrete cond type. `matches()` dispatches to the
/// correct `EventCond::matches()` implementation based on the variant.
#[derive(Debug, Clone, PartialEq)]
pub enum Cond {
    Wildcard,
    MetadataRawTreeReady(OnMetadataRawTreeReadyEventCond),
    MetadataLocalTreeReady(OnMetadataLocalTreeReadyEventCond),
    ServerError(OnServerErrorEventCond),
    /// Events with `EventPayload<(), ()>` have no cond.
    /// `matches` always returns `true`.
    Unit,
}

impl Cond {
    pub fn matches(&self, dispatch: &Cond) -> bool {
        match (self, dispatch) {
            (Cond::Wildcard, _) => true,
            (Cond::MetadataRawTreeReady(a), Cond::MetadataRawTreeReady(b)) => a.matches(b),
            (Cond::MetadataLocalTreeReady(a), Cond::MetadataLocalTreeReady(b)) => a.matches(b),
            (Cond::ServerError(a), Cond::ServerError(b)) => a.matches(b),
            (Cond::Unit, Cond::Unit) => true,
            _ => false,
        }
    }
}
