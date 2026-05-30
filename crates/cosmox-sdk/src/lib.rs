use std::pin::Pin;

use crate::types::{
    InitStatus, InitializeConfig, InstallPlugin, Library, LibraryAdd, LibraryDeleteRequest,
    LibraryModify, LibraryQueryRequest, LibraryType, Permission, PermissionAddRequest,
    PushResponse, Resource, ResourceAddRequest, ResourceModifyRequest, ResourceQueryRequest, Role,
    RoleAddRequest, RoleLinkPermissionAddRequest, ScannerInfo, ScannerStatus,
    ScannerTaskAddRequest, SearchRequest, SystemInfo, Tag, TagAddRequest, TagCatalogEntry,
    TagGroup, TagGroupAddRequest, TagGroupDeleteRequest, TagGroupQueryRequest, TagQueryRequest,
    User, UserLogin, UserQueryRequest, UserResp, UserRoleAddRequest, UserSignUp,
};

/// Available backend transports
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Backend {
    Web,
    Ipc,
    Direct,
}

/// Returns the list of backends compiled into this build.
pub fn available_backends() -> Vec<Backend> {
    let mut backends = Vec::new();
    #[cfg(feature = "web")]
    backends.push(Backend::Web);
    #[cfg(feature = "ipc")]
    backends.push(Backend::Ipc);
    #[cfg(feature = "direct")]
    backends.push(Backend::Direct);
    backends
}

/// Create a client for the given backend.
/// # Panics
/// If the backend's transport feature is not enabled at compile time.
#[cfg(any(feature = "web", feature = "ipc", feature = "direct"))]
#[cfg_attr(
    all(feature = "web", feature = "ipc", feature = "direct"),
    allow(unreachable_patterns)
)]
pub fn create_client(backend: Backend, hostname: &'static str, port: u16) -> Box<dyn Api> {
    assert!(
        url::Host::parse(hostname).is_ok(),
        "Failed to parse hostname '{hostname}': must be a valid domain name, IPv4, or IPv6 address."
    );

    match backend {
        Backend::Web => Box::new(transport::web::HttpApi::new(hostname, port)),
        #[cfg(feature = "ipc")]
        Backend::Ipc => Box::new(transport::ipc::IpcClient::new(hostname, port)),
        #[cfg(feature = "direct")]
        Backend::Direct => Box::new(transport::direct::DirectClient::new(hostname, port)),
        _ => panic!("create_client: backend feature not enabled at compile time"),
    }
}

// -- Core API trait (dyn-safe with Pin<Box<dyn Future>>) --

#[cfg(any(feature = "native", feature = "ffi"))]
pub trait Api {
    fn new(hostname: &'static str, port: u16) -> Self
    where
        Self: Sized;

    // -- Auth --
    fn set_token(&mut self, token: String);
    fn get_token(&self) -> Option<String>;
    fn logout(&mut self);
    fn login(
        &mut self,
        payload: UserLogin,
    ) -> Pin<Box<dyn Future<Output = Result<(), SdkError>> + Send + '_>>;

    // -- System --
    fn system_info(
        &self,
    ) -> Pin<Box<dyn Future<Output = Result<SystemInfo, SdkError>> + Send + '_>>;
    fn system_about(&self) -> Pin<Box<dyn Future<Output = Result<String, SdkError>> + Send + '_>>;
    fn system_log(&self) -> Pin<Box<dyn Future<Output = Result<String, SdkError>> + Send + '_>>;
    fn system_restart(&self) -> Pin<Box<dyn Future<Output = Result<(), SdkError>> + Send + '_>>;
    fn system_shutdown(&self) -> Pin<Box<dyn Future<Output = Result<(), SdkError>> + Send + '_>>;
    fn system_delete_all(&self) -> Pin<Box<dyn Future<Output = Result<(), SdkError>> + Send + '_>>;

    // -- User --
    fn user_get(
        &self,
        uid: u64,
    ) -> Pin<Box<dyn Future<Output = Result<User, SdkError>> + Send + '_>>;
    fn user_query(
        &self,
        params: UserQueryRequest,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<User>, SdkError>> + Send + '_>>;
    fn user_register(
        &self,
        payload: UserSignUp,
    ) -> Pin<Box<dyn Future<Output = Result<UserResp, SdkError>> + Send + '_>>;
    fn user_delete(
        &self,
        uid: u64,
    ) -> Pin<Box<dyn Future<Output = Result<(), SdkError>> + Send + '_>>;
    fn user_role_add(
        &self,
        payload: UserRoleAddRequest,
    ) -> Pin<Box<dyn Future<Output = Result<(), SdkError>> + Send + '_>>;

    // -- Library --
    fn library_get(
        &self,
        lid: u64,
    ) -> Pin<Box<dyn Future<Output = Result<Library, SdkError>> + Send + '_>>;
    fn library_query(
        &self,
        params: LibraryQueryRequest,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<Library>, SdkError>> + Send + '_>>;
    fn library_add(
        &self,
        payload: LibraryAdd,
    ) -> Pin<Box<dyn Future<Output = Result<serde_json::Value, SdkError>> + Send + '_>>;
    fn library_modify(
        &self,
        lid: u64,
        payload: LibraryModify,
    ) -> Pin<Box<dyn Future<Output = Result<(), SdkError>> + Send + '_>>;
    fn library_delete(
        &self,
        payload: LibraryDeleteRequest,
    ) -> Pin<Box<dyn Future<Output = Result<(), SdkError>> + Send + '_>>;
    fn library_type_all(
        &self,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<LibraryType>, SdkError>> + Send + '_>>;

    // -- Tag --
    fn tag_get(&self, tid: u64)
    -> Pin<Box<dyn Future<Output = Result<Tag, SdkError>> + Send + '_>>;
    fn tag_add(
        &self,
        payload: TagAddRequest,
    ) -> Pin<Box<dyn Future<Output = Result<u64, SdkError>> + Send + '_>>;
    fn tag_delete(
        &self,
        tid: u64,
    ) -> Pin<Box<dyn Future<Output = Result<(), SdkError>> + Send + '_>>;
    fn tag_query(
        &self,
        params: TagQueryRequest,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<Tag>, SdkError>> + Send + '_>>;

    // -- Tag Group --
    fn tag_group_get(
        &self,
        tgid: u64,
    ) -> Pin<Box<dyn Future<Output = Result<TagGroup, SdkError>> + Send + '_>>;
    fn tag_group_add(
        &self,
        payload: TagGroupAddRequest,
    ) -> Pin<Box<dyn Future<Output = Result<u64, SdkError>> + Send + '_>>;
    fn tag_group_delete(
        &self,
        payload: TagGroupDeleteRequest,
    ) -> Pin<Box<dyn Future<Output = Result<(), SdkError>> + Send + '_>>;
    fn tag_group_query(
        &self,
        params: TagGroupQueryRequest,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<TagGroup>, SdkError>> + Send + '_>>;
    fn tag_catalog(
        &self,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<TagCatalogEntry>, SdkError>> + Send + '_>>;

    // -- Resource --
    fn resource_get(
        &self,
        rid: u64,
    ) -> Pin<Box<dyn Future<Output = Result<Resource, SdkError>> + Send + '_>>;
    fn resource_query(
        &self,
        params: ResourceQueryRequest,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<Resource>, SdkError>> + Send + '_>>;
    fn resource_add(
        &self,
        payload: ResourceAddRequest,
    ) -> Pin<Box<dyn Future<Output = Result<u64, SdkError>> + Send + '_>>;
    fn resource_modify(
        &self,
        rid: u64,
        payload: ResourceModifyRequest,
    ) -> Pin<Box<dyn Future<Output = Result<(), SdkError>> + Send + '_>>;
    fn resource_delete(
        &self,
        rid: u64,
    ) -> Pin<Box<dyn Future<Output = Result<(), SdkError>> + Send + '_>>;
    fn resource_add_tag(
        &self,
        rid: u64,
        tag_ids: Vec<u64>,
    ) -> Pin<Box<dyn Future<Output = Result<serde_json::Value, SdkError>> + Send + '_>>;
    fn resource_remove_tag(
        &self,
        rid: u64,
        tag_ids: Vec<u64>,
    ) -> Pin<Box<dyn Future<Output = Result<(), SdkError>> + Send + '_>>;
    fn resource_get_metadata(
        &self,
        rid: u64,
    ) -> Pin<Box<dyn Future<Output = Result<serde_json::Value, SdkError>> + Send + '_>>;

    // -- Acl --
    fn acl_query_role(
        &self,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<Role>, SdkError>> + Send + '_>>;
    fn acl_query_permission(
        &self,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<Permission>, SdkError>> + Send + '_>>;
    fn acl_add_role(
        &self,
        payload: RoleAddRequest,
    ) -> Pin<Box<dyn Future<Output = Result<(), SdkError>> + Send + '_>>;
    fn acl_delete_role(
        &self,
        rid: u64,
    ) -> Pin<Box<dyn Future<Output = Result<(), SdkError>> + Send + '_>>;
    fn acl_add_permission(
        &self,
        payload: PermissionAddRequest,
    ) -> Pin<Box<dyn Future<Output = Result<(), SdkError>> + Send + '_>>;
    fn acl_delete_permission(
        &self,
        pid: u64,
    ) -> Pin<Box<dyn Future<Output = Result<(), SdkError>> + Send + '_>>;
    fn acl_add_permission_for_role(
        &self,
        payload: RoleLinkPermissionAddRequest,
    ) -> Pin<Box<dyn Future<Output = Result<(), SdkError>> + Send + '_>>;

    // -- Plugin --
    fn plugin_info(&self) -> Pin<Box<dyn Future<Output = Result<String, SdkError>> + Send + '_>>;
    fn plugin_install(
        &self,
        payload: InstallPlugin,
    ) -> Pin<Box<dyn Future<Output = Result<String, SdkError>> + Send + '_>>;
    fn plugin_uninstall(
        &self,
        id: u64,
    ) -> Pin<Box<dyn Future<Output = Result<(), SdkError>> + Send + '_>>;
    fn plugin_enable(
        &self,
        id: u64,
    ) -> Pin<Box<dyn Future<Output = Result<(), SdkError>> + Send + '_>>;
    fn plugin_disable(
        &self,
        id: u64,
    ) -> Pin<Box<dyn Future<Output = Result<(), SdkError>> + Send + '_>>;

    // -- Scanner --
    fn scanner_scan(
        &self,
        lid: u64,
    ) -> Pin<Box<dyn Future<Output = Result<String, SdkError>> + Send + '_>>;
    fn scanner_scan_all(
        &self,
    ) -> Pin<Box<dyn Future<Output = Result<String, SdkError>> + Send + '_>>;
    fn scanner_get_status(
        &self,
    ) -> Pin<Box<dyn Future<Output = Result<ScannerStatus, SdkError>> + Send + '_>>;
    fn scanner_info(
        &self,
    ) -> Pin<Box<dyn Future<Output = Result<ScannerInfo, SdkError>> + Send + '_>>;
    fn scanner_add_task(
        &self,
        payload: ScannerTaskAddRequest,
    ) -> Pin<Box<dyn Future<Output = Result<(), SdkError>> + Send + '_>>;

    // -- Metadata --
    fn metadata_query(
        &self,
        root_node: u64,
        depth: usize,
    ) -> Pin<Box<dyn Future<Output = Result<serde_json::Value, SdkError>> + Send + '_>>;
    fn metadata_get(
        &self,
        rid: u64,
    ) -> Pin<Box<dyn Future<Output = Result<serde_json::Value, SdkError>> + Send + '_>>;

    // -- Path --
    fn path_sub_path(
        &self,
        path: String,
        show_hide: bool,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<String>, SdkError>> + Send + '_>>;

    // -- Init --
    fn initialize(
        &self,
        payload: InitializeConfig,
    ) -> Pin<Box<dyn Future<Output = Result<InitStatus, SdkError>> + Send + '_>>;

    // -- File / Avatar --
    fn user_upload_avatar(
        &self,
        uid: u64,
        data: Vec<u8>,
    ) -> Pin<Box<dyn Future<Output = Result<PushResponse, SdkError>> + Send + '_>>;
    fn item_push(
        &self,
        data: Vec<u8>,
    ) -> Pin<Box<dyn Future<Output = Result<PushResponse, SdkError>> + Send + '_>>;
    fn item_pull(
        &self,
        id: u64,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<u8>, SdkError>> + Send + '_>>;

    // -- Search --
    fn search(
        &self,
        query: SearchRequest,
    ) -> Pin<Box<dyn Future<Output = Result<serde_json::Value, SdkError>> + Send + '_>>;

    // -- Docs --
    fn openapi(&self) -> Pin<Box<dyn Future<Output = Result<String, SdkError>> + Send + '_>>;
    fn docs(&self) -> Pin<Box<dyn Future<Output = Result<String, SdkError>> + Send + '_>>;
}

// -- Modules --

#[cfg(feature = "ffi")]
pub mod ffi;

#[cfg(any(feature = "web", feature = "ipc", feature = "direct"))]
pub mod transport;

pub mod error;
pub mod types;

pub use error::SdkError;

#[cfg(feature = "web")]
pub use transport::web::HttpApi;
