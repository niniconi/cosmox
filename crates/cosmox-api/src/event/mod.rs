use anyhow::{Result, anyhow};
use bincode::{Decode, Encode};

use crate::event::payloads::{
    OnMetadataLocalTreeReadyEventCond, OnMetadataLocalTreeReadyEventContext,
    OnMetadataRawTreeReadyEventCond, OnMetadataRawTreeReadyEventContext, OnServerErrorEventCond,
    OnServerErrorEventContext,
};

pub mod payloads;

/// Defines user interactions and system events that can occur in a multimedia management system.
///
/// This enum covers a wide range of events, including user authentication, media file management, playback,
/// interaction, and various system-level occurrences.
#[derive(Debug, Encode, Decode)]
pub enum Event {
    OnMetadataRawTreeReady(
        EventPayload<OnMetadataRawTreeReadyEventCond, OnMetadataRawTreeReadyEventContext>,
    ),
    OnMetadataLocalTreeReady(
        EventPayload<OnMetadataLocalTreeReadyEventCond, OnMetadataLocalTreeReadyEventContext>,
    ),

    OnScanComplete(EventPayload<(), ()>),
    OnNewFileDiscovered(EventPayload<(), ()>),

    OnUserLogin(EventPayload<(), ()>),
    OnLibraryCrate(EventPayload<(), ()>),

    OnPluginInstall(EventPayload<(), ()>),
    OnPluginUninstall(EventPayload<(), ()>),
    OnPluginEnable(EventPayload<(), ()>),
    OnPluginDisable(EventPayload<(), ()>),

    OnServerStart(EventPayload<(), ()>),
    OnServerStop(EventPayload<(), ()>),
    OnServerError(EventPayload<OnServerErrorEventCond, OnServerErrorEventContext>),
}

#[derive(Debug, Encode, Decode)]
pub enum EventPayload<C, D> {
    /// The filter condition for plugin event registration;
    /// the event is triggered only when the condition is met.
    Cond(C),
    /// The plugin receives the event context data after the event is triggered.
    Data(D),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct EventKey(std::mem::Discriminant<Event>);

impl Event {
    pub fn encode(&self) -> Result<Vec<u8>> {
        let config = bincode::config::standard();
        bincode::encode_to_vec(self, config).map_err(|err| anyhow!(err))
    }

    pub fn decode(data: Vec<u8>) -> Result<Event> {
        let config = bincode::config::standard();
        bincode::decode_from_slice::<Event, _>(data.as_slice(), config)
            .map(|(event, _)| event)
            .map_err(|err| anyhow!(err))
    }

    #[cfg(feature = "plugin")]
    pub fn register(&self) -> Result<()> {
        super::api::bindings::cosmox::plugin::cosmox_api::register_event_listener(
            self.encode()?.as_slice(),
        )?;
        Ok(())
    }
}

impl PartialEq for Event {
    fn eq(&self, other: &Self) -> bool {
        self.into_key() == other.into_key()
    }
}
impl Event {
    pub fn into_key(&self) -> EventKey {
        EventKey(std::mem::discriminant(self))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn event_decode_and_encode() {
        let event: Event = Event::OnServerStart(EventPayload::Data(()));
        let data = event.encode().unwrap();
        let event_decode = Event::decode(data).unwrap();
        assert_eq!(event, event_decode)
    }
}
