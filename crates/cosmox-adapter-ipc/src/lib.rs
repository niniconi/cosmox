//! # cosmox-adapter-ipc
//!
//! IPC server for cosmox using Unix domain sockets.
//!
//! ## Protocol
//!
//! All frames use rkyv serialization with a length-prefixed wire format:
//!
//! ```text
//! [u64 LE frame_length][rkyv_bytes(IpcRequest)]
//! [u64 LE frame_length][rkyv_bytes(IpcResponse)]
//! ```
//!
//! ### Request
//!
//! - `endpoint`: `IpcEndpoint` enum identifying the API endpoint
//! - `token`: Optional JWT token for authentication
//! - `payload`: rkyv-serialized endpoint-specific parameters
//!
//! ### Response
//!
//! - `success`: Boolean indicating if the request succeeded
//! - `data`: Optional rkyv-serialized response data (present on success)
//! - `error`: Optional error message (present on failure)

mod handler;
mod protocol;
mod server;

pub use protocol::{IpcEndpoint, IpcRequest, IpcResponse};
pub use server::server;

#[cfg(test)]
mod tests {
    use rkyv::rancor;
    use tokio::io::AsyncWriteExt;

    use super::*;

    #[test]
    fn test_ipc_request_roundtrip() {
        let req = IpcRequest {
            endpoint: IpcEndpoint::GetSystemInfo,
            token: Some("test-token".to_string()),
            payload: vec![1, 2, 3, 4],
        };

        let bytes = rkyv::to_bytes::<rancor::Error>(&req).unwrap();
        let archived = rkyv::from_bytes::<IpcRequest, rancor::Error>(&bytes).unwrap();

        assert_eq!(archived.endpoint, IpcEndpoint::GetSystemInfo);
        assert_eq!(archived.token, Some("test-token".to_string()));
        assert_eq!(archived.payload, vec![1, 2, 3, 4]);
    }

    #[test]
    fn test_ipc_response_roundtrip() {
        let resp = IpcResponse {
            success: true,
            data: Some(vec![10, 20, 30]),
            error: None,
        };

        let bytes = rkyv::to_bytes::<rancor::Error>(&resp).unwrap();
        let archived = rkyv::from_bytes::<IpcResponse, rancor::Error>(&bytes).unwrap();

        assert!(archived.success);
        assert_eq!(archived.data, Some(vec![10, 20, 30]));
        assert!(archived.error.is_none());
    }

    #[test]
    fn test_ipc_response_error_roundtrip() {
        let resp = IpcResponse::error("something went wrong");

        let bytes = rkyv::to_bytes::<rancor::Error>(&resp).unwrap();
        let archived = rkyv::from_bytes::<IpcResponse, rancor::Error>(&bytes).unwrap();

        assert!(!archived.success);
        assert!(archived.data.is_none());
        assert_eq!(archived.error, Some("something went wrong".to_string()));
    }

    #[test]
    fn test_all_endpoint_variants() {
        let variants: Vec<IpcEndpoint> = vec![
            IpcEndpoint::GetSystemInfo,
            IpcEndpoint::GetSystemAbout,
            IpcEndpoint::GetSystemLog,
            IpcEndpoint::SystemShutdown,
            IpcEndpoint::SystemRestart,
            IpcEndpoint::GetLibrary(1),
            IpcEndpoint::QueryLibrary,
            IpcEndpoint::ModifyLibrary(42),
            IpcEndpoint::AddLibrary,
            IpcEndpoint::DeleteLibrary(7),
            IpcEndpoint::GetAllLibraryTypes,
            IpcEndpoint::GetTag(100),
            IpcEndpoint::GetTagGroup(200),
            IpcEndpoint::GetTagCatalog,
            IpcEndpoint::QueryTag,
            IpcEndpoint::AddTag,
            IpcEndpoint::QueryTagGroup,
            IpcEndpoint::AddTagGroup,
            IpcEndpoint::DeleteTagGroup(300),
            IpcEndpoint::GetResource(400),
            IpcEndpoint::AddResource,
            IpcEndpoint::DeleteResource(500),
            IpcEndpoint::QueryResource,
            IpcEndpoint::AddTagForResource(600),
            IpcEndpoint::GetUser(700),
            IpcEndpoint::QueryUser,
            IpcEndpoint::ScanLibrary(800),
            IpcEndpoint::ScanAll,
            IpcEndpoint::Search,
            IpcEndpoint::Initialize,
            IpcEndpoint::GetSubPath,
            IpcEndpoint::GetRole(900),
            IpcEndpoint::QueryRole,
            IpcEndpoint::AddRole,
            IpcEndpoint::DeleteRole(1000),
            IpcEndpoint::GetRolesByUser(1100),
            IpcEndpoint::GetPermission(1200),
            IpcEndpoint::QueryPermission,
            IpcEndpoint::AddPermission,
            IpcEndpoint::DeletePermission(1300),
            IpcEndpoint::GetPermissionsByRole(1400),
            IpcEndpoint::GetPermissionsByUser(1500),
            IpcEndpoint::AddPermissionForRole(1600, 1700),
            IpcEndpoint::AddRoleForUser(1800, 1900),
            IpcEndpoint::PluginInfo,
        ];

        for ep in variants {
            let req = IpcRequest {
                endpoint: ep,
                token: None,
                payload: vec![],
            };
            let bytes = rkyv::to_bytes::<rancor::Error>(&req).unwrap();
            let archived = rkyv::from_bytes::<IpcRequest, rancor::Error>(&bytes).unwrap();
            assert_eq!(archived.endpoint, req.endpoint);
            assert_eq!(archived.payload, req.payload);
            assert!(archived.token.is_none());
        }
    }

    #[tokio::test]
    async fn test_async_write_read_frame() {
        let payload = b"hello async ipc";
        let mut buf = Vec::new();
        crate::protocol::write_frame(&mut buf, payload)
            .await
            .unwrap();
        let (mut tx, mut rx) = tokio::io::duplex(1024);
        tx.write_all(&buf).await.unwrap();
        drop(tx);
        let result = crate::protocol::read_frame(&mut rx).await.unwrap();
        assert_eq!(result, payload);
    }

    #[tokio::test]
    async fn test_async_write_read_frame_roundtrip() {
        let payload = b"roundtrip data 12345";
        let mut buf = Vec::new();
        crate::protocol::write_frame(&mut buf, payload)
            .await
            .unwrap();
        let (mut tx, mut rx) = tokio::io::duplex(4096);
        tx.write_all(&buf).await.unwrap();
        drop(tx);
        let result = crate::protocol::read_frame(&mut rx).await.unwrap();
        assert_eq!(result, payload);
    }

    #[tokio::test]
    async fn test_async_empty_frame() {
        let payload = b"";
        let mut buf = Vec::new();
        crate::protocol::write_frame(&mut buf, payload)
            .await
            .unwrap();
        let (mut tx, mut rx) = tokio::io::duplex(1024);
        tx.write_all(&buf).await.unwrap();
        drop(tx);
        let result = crate::protocol::read_frame(&mut rx).await.unwrap();
        assert_eq!(result, payload);
    }

    #[tokio::test]
    async fn test_async_large_frame() {
        let payload = vec![0xABu8; 65_536];
        let mut buf = Vec::new();
        crate::protocol::write_frame(&mut buf, &payload)
            .await
            .unwrap();
        let (mut tx, mut rx) = tokio::io::duplex(131_072);
        tx.write_all(&buf).await.unwrap();
        drop(tx);
        let result = crate::protocol::read_frame(&mut rx).await.unwrap();
        assert_eq!(result, payload);
    }

    #[tokio::test]
    async fn test_multi_request_frame_sequence() {
        let requests = vec![
            IpcRequest {
                endpoint: IpcEndpoint::GetSystemInfo,
                token: Some("tok1".into()),
                payload: vec![],
            },
            IpcRequest {
                endpoint: IpcEndpoint::GetSystemAbout,
                token: None,
                payload: vec![1, 2, 3],
            },
            IpcRequest {
                endpoint: IpcEndpoint::SystemShutdown,
                token: Some("tok2".into()),
                payload: vec![4, 5, 6, 7, 8],
            },
        ];

        let mut buf = Vec::new();
        for req in &requests {
            let ser = rkyv::to_bytes::<rancor::Error>(req).unwrap();
            let bytes: &[u8] = &ser;
            crate::protocol::write_frame(&mut buf, bytes).await.unwrap();
        }

        let (mut tx, mut rx) = tokio::io::duplex(4096);
        tx.write_all(&buf).await.unwrap();
        drop(tx);

        for expected in &requests {
            let frame = crate::protocol::read_frame(&mut rx).await.unwrap();
            let archived = rkyv::from_bytes::<IpcRequest, rancor::Error>(&frame).unwrap();
            assert_eq!(archived.endpoint, expected.endpoint);
            assert_eq!(archived.token, expected.token);
            assert_eq!(archived.payload, expected.payload);
        }
    }

    #[test]
    fn test_deserialize_invalid_data() {
        let bogus = vec![0xFF, 0xFF, 0xFF, 0xFF];
        assert!(rkyv::from_bytes::<IpcRequest, rancor::Error>(&bogus).is_err());
    }

    #[test]
    fn test_deserialize_empty_data() {
        let empty: Vec<u8> = vec![];
        assert!(rkyv::from_bytes::<IpcRequest, rancor::Error>(&empty).is_err());
    }

    #[tokio::test]
    async fn test_read_frame_incomplete() {
        let buf = vec![0x05, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01, 0x02];
        let (mut tx, mut rx) = tokio::io::duplex(64);
        tx.write_all(&buf).await.unwrap();
        drop(tx);
        let result = crate::protocol::read_frame(&mut rx).await;
        assert!(result.is_err());
    }

    #[test]
    fn test_serialize_response_roundtrip() {
        let resp = IpcResponse {
            success: true,
            data: Some(b"test response".to_vec()),
            error: None,
        };
        let bytes = crate::protocol::serialize_response(&resp).unwrap();
        let archived = rkyv::from_bytes::<IpcResponse, rancor::Error>(&bytes).unwrap();
        assert!(archived.success);
        assert_eq!(archived.data, resp.data);
    }

    #[test]
    fn test_deserialize_request_roundtrip() {
        let req = IpcRequest {
            endpoint: IpcEndpoint::GetTagCatalog,
            token: Some("catalog-token".into()),
            payload: b"raw payload".to_vec(),
        };
        let ser = rkyv::to_bytes::<rancor::Error>(&req).unwrap();
        let deserialized = crate::protocol::deserialize_request(&ser).unwrap();
        assert_eq!(deserialized.endpoint, IpcEndpoint::GetTagCatalog);
        assert_eq!(deserialized.token, req.token);
        assert_eq!(deserialized.payload, req.payload);
    }
}
