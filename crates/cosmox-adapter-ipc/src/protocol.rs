//! IPC protocol types — frame format, endpoint enum, request/response structs.

use std::io;

use rkyv::{Archive, Deserialize, Serialize, from_bytes, rancor, to_bytes};

/// All IPC endpoints.
#[derive(Archive, Serialize, Deserialize, Debug, Clone, PartialEq)]
#[rkyv(bytecheck())]
pub enum IpcEndpoint {
    // System
    GetSystemInfo,
    GetSystemAbout,
    GetSystemLog,
    SystemShutdown,
    SystemRestart,
    // Library
    GetLibrary(u64),
    QueryLibrary,
    ModifyLibrary(u64),
    AddLibrary,
    DeleteLibrary(u64),
    GetAllLibraryTypes,
    // Tag
    GetTag(u64),
    GetTagGroup(u64),
    GetTagCatalog,
    QueryTag,
    AddTag,
    QueryTagGroup,
    AddTagGroup,
    DeleteTagGroup(u64),
    // Resource
    GetResource(u64),
    AddResource,
    DeleteResource(u64),
    QueryResource,
    AddTagForResource(u64),
    // User
    GetUser(u64),
    QueryUser,
    // Scanner
    ScanLibrary(u64),
    ScanAll,
    // Search
    Search,
    // Init
    Initialize,
    // Path Tree
    GetSubPath,
    // ACL
    GetRole(u64),
    QueryRole,
    AddRole,
    DeleteRole(u64),
    GetRolesByUser(u64),
    GetPermission(u64),
    QueryPermission,
    AddPermission,
    DeletePermission(u64),
    GetPermissionsByRole(u64),
    GetPermissionsByUser(u64),
    AddPermissionForRole(u64, u64),
    AddRoleForUser(u64, u64),
    // Plugin
    PluginInfo,
}

#[derive(Archive, Serialize, Deserialize, Debug)]
#[rkyv(bytecheck())]
pub struct IpcRequest {
    pub endpoint: IpcEndpoint,
    pub token: Option<String>,
    pub payload: Vec<u8>,
}

#[derive(Archive, Serialize, Deserialize, Debug)]
#[rkyv(bytecheck())]
pub struct IpcResponse {
    pub success: bool,
    pub data: Option<Vec<u8>>,
    pub error: Option<String>,
}

impl IpcResponse {
    pub fn ok(data: Vec<u8>) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
        }
    }

    pub fn error(msg: impl Into<String>) -> Self {
        Self {
            success: false,
            data: None,
            error: Some(msg.into()),
        }
    }
}

pub const REQUEST_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(30);

pub async fn read_frame(
    reader: &mut (impl tokio::io::AsyncReadExt + Unpin),
) -> io::Result<Vec<u8>> {
    let mut len_buf = [0u8; 8];
    reader.read_exact(&mut len_buf).await?;
    let len = u64::from_le_bytes(len_buf) as usize;
    let mut buf = vec![0u8; len];
    reader.read_exact(&mut buf).await?;
    Ok(buf)
}

pub async fn write_frame(
    writer: &mut (impl tokio::io::AsyncWriteExt + Unpin),
    data: &[u8],
) -> io::Result<()> {
    let len = data.len() as u64;
    writer.write_all(&len.to_le_bytes()).await?;
    writer.write_all(data).await?;
    writer.flush().await?;
    Ok(())
}

pub fn serialize_response(resp: &IpcResponse) -> Result<Vec<u8>, String> {
    to_bytes::<rancor::Error>(resp)
        .map(|aligned| aligned.into_vec())
        .map_err(|e| format!("rkyv serialization failed: {e}"))
}

pub fn deserialize_request(buf: &[u8]) -> Result<IpcRequest, String> {
    from_bytes::<IpcRequest, rancor::Error>(buf)
        .map_err(|e| format!("rkyv deserialization failed: {e}"))
}
