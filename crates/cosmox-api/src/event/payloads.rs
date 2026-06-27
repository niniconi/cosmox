use bincode::{Decode, Encode};

#[derive(Debug, Encode, Decode)]
pub struct OnMetadataRawTreeReadyEventContext {
    /// library id
    pub lid: u64,
    /// library type
    pub r#type: String,
}

#[derive(Debug, Encode, Decode)]
pub struct OnMetadataRawTreeReadyEventCond {
    /// library type
    pub r#type: Vec<String>,
}

#[derive(Debug, Encode, Decode)]
pub struct OnMetadataLocalTreeReadyEventContext {
    /// library id
    pub lid: u64,
    /// library type
    pub r#type: String,
    /// Indicates which plugins have already processed this metadata tree.
    pub from_plugins: Vec<String>,
}

#[derive(Debug, Encode, Decode)]
pub struct OnMetadataLocalTreeReadyEventCond {
    /// library type
    pub r#type: Vec<String>,
    /// Excludes metadata already processed by specific plugins.
    pub exclude_from_plugins: Vec<String>,
}

#[derive(Debug, Encode, Decode)]
pub struct OnServerErrorEventContext {
    pub errors: Vec<String>,
}

#[derive(Debug, Encode, Decode)]
pub struct OnServerErrorEventCond {
    pub level: String,
}
