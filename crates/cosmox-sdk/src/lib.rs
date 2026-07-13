use std::pin::Pin;

/// Shorthand for a pinned, boxed, `Send` future returned by API methods.
type ApiFuture<'a, T> = Pin<Box<dyn Future<Output = Result<T, SdkError>> + Send + 'a>>;

use crate::types::{
    InitStatus, InitializeConfig, InstallPlugin, LibrariesRelatedTags, Library, LibraryAdd,
    LibraryDeleteRequest, LibraryModify, LibraryPath, LibraryQueryRequest, LibraryType, Permission,
    PermissionAddRequest, PushResponse, Resource, ResourceAddRequest, ResourceModifyRequest,
    ResourceQueryRequest, Role, RoleAddRequest, RoleLinkPermissionAddRequest, ScannerInfo,
    ScannerStatus, ScannerTaskAddRequest, SearchRequest, SystemInfo, Tag, TagAddRequest,
    TagCatalogEntry, TagGroup, TagGroupAddRequest, TagGroupDeleteRequest, TagGroupQueryRequest,
    TagQueryRequest, User, UserLogin, UserQueryRequest, UserResp, UserRoleAddRequest, UserSignUp,
};

pub use error::SdkError;

#[cfg(feature = "direct")]
pub use transport::direct::DirectApi;
#[cfg(feature = "ipc")]
pub use transport::ipc::IpcApi;
#[cfg(feature = "web")]
pub use transport::web::HttpApi;

#[cfg(feature = "ffi")]
pub mod ffi;

#[cfg(any(feature = "web", feature = "ipc", feature = "direct"))]
pub(crate) mod transport;

pub(crate) mod error;
pub mod types;

/// Create a client for the given generic parameter.
#[cfg(any(feature = "web", feature = "ipc", feature = "direct"))]
pub fn create_client<T: Api>(hostname: &'static str, port: u16) -> T {
    assert!(
        url::Host::parse(hostname).is_ok(),
        "Failed to parse hostname '{hostname}': must be a valid domain name, IPv4, or IPv6 address."
    );

    T::new(hostname, port)
}

pub trait Api {
    fn new(hostname: &'static str, port: u16) -> Self
    where
        Self: Sized;

    fn set_token(&mut self, token: String);
    fn get_token(&self) -> Option<String>;
    fn logout(&mut self);
    fn login(&mut self, payload: UserLogin) -> ApiFuture<'_, ()>;

    fn system_info(&self) -> ApiFuture<'_, SystemInfo>;
    fn system_about(&self) -> ApiFuture<'_, String>;
    fn system_log(&self) -> ApiFuture<'_, String>;
    fn system_restart(&self) -> ApiFuture<'_, ()>;
    fn system_shutdown(&self) -> ApiFuture<'_, ()>;
    fn system_delete_all(&self) -> ApiFuture<'_, ()>;

    fn user_get(&self, uid: u64) -> ApiFuture<'_, User>;
    fn user_query(&self, params: UserQueryRequest) -> ApiFuture<'_, Vec<User>>;
    fn user_register(&self, payload: UserSignUp) -> ApiFuture<'_, UserResp>;
    fn user_delete(&self, uid: u64) -> ApiFuture<'_, ()>;
    fn user_role_add(&self, payload: UserRoleAddRequest) -> ApiFuture<'_, ()>;

    fn library_get(&self, lid: u64) -> ApiFuture<'_, Library>;
    fn library_query(&self, params: LibraryQueryRequest) -> ApiFuture<'_, Vec<Library>>;
    fn library_add(
        &self,
        payload: LibraryAdd,
    ) -> ApiFuture<'_, (Library, Vec<LibrariesRelatedTags>, Vec<LibraryPath>)>;
    fn library_modify(&self, lid: u64, payload: LibraryModify) -> ApiFuture<'_, ()>;
    fn library_delete(&self, payload: LibraryDeleteRequest) -> ApiFuture<'_, ()>;
    fn library_type_all(&self) -> ApiFuture<'_, Vec<LibraryType>>;

    fn tag_get(&self, tid: u64) -> ApiFuture<'_, Tag>;
    fn tag_add(&self, payload: TagAddRequest) -> ApiFuture<'_, u64>;
    fn tag_query(&self, params: TagQueryRequest) -> ApiFuture<'_, Vec<Tag>>;

    fn tag_group_get(&self, tgid: u64) -> ApiFuture<'_, TagGroup>;
    fn tag_group_add(&self, payload: TagGroupAddRequest) -> ApiFuture<'_, u64>;
    fn tag_group_delete(&self, payload: TagGroupDeleteRequest) -> ApiFuture<'_, ()>;
    fn tag_group_query(&self, params: TagGroupQueryRequest) -> ApiFuture<'_, Vec<TagGroup>>;
    fn tag_catalog(&self) -> ApiFuture<'_, Vec<TagCatalogEntry>>;

    fn resource_get(&self, rid: u64) -> ApiFuture<'_, Resource>;
    fn resource_query(&self, params: ResourceQueryRequest) -> ApiFuture<'_, Vec<Resource>>;
    fn resource_add(&self, payload: ResourceAddRequest) -> ApiFuture<'_, u64>;
    fn resource_modify(&self, rid: u64, payload: ResourceModifyRequest) -> ApiFuture<'_, ()>;
    fn resource_delete(&self, rid: u64) -> ApiFuture<'_, ()>;
    fn resource_add_tag(&self, rid: u64, tag_ids: Vec<u64>) -> ApiFuture<'_, serde_json::Value>;
    fn resource_get_metadata(&self, rid: u64) -> ApiFuture<'_, serde_json::Value>;

    fn acl_query_role(&self) -> ApiFuture<'_, Vec<Role>>;
    fn acl_query_permission(&self) -> ApiFuture<'_, Vec<Permission>>;
    fn acl_add_role(&self, payload: RoleAddRequest) -> ApiFuture<'_, ()>;
    fn acl_delete_role(&self, rid: u64) -> ApiFuture<'_, ()>;
    fn acl_add_permission(&self, payload: PermissionAddRequest) -> ApiFuture<'_, ()>;
    fn acl_delete_permission(&self, pid: u64) -> ApiFuture<'_, ()>;
    fn acl_add_permission_for_role(
        &self,
        payload: RoleLinkPermissionAddRequest,
    ) -> ApiFuture<'_, ()>;

    fn plugin_info(&self) -> ApiFuture<'_, String>;
    fn plugin_install(&self, payload: InstallPlugin) -> ApiFuture<'_, String>;
    fn plugin_uninstall(&self, name: String) -> ApiFuture<'_, ()>;
    fn plugin_enable(&self, name: String) -> ApiFuture<'_, ()>;
    fn plugin_disable(&self, name: String) -> ApiFuture<'_, ()>;

    fn scanner_scan(&self, lid: u64) -> ApiFuture<'_, String>;
    fn scanner_scan_all(&self) -> ApiFuture<'_, String>;
    fn scanner_get_status(&self) -> ApiFuture<'_, ScannerStatus>;
    fn scanner_info(&self) -> ApiFuture<'_, ScannerInfo>;
    fn scanner_add_task(&self, payload: ScannerTaskAddRequest) -> ApiFuture<'_, ()>;

    fn metadata_query(&self, root_node: u64, depth: usize) -> ApiFuture<'_, serde_json::Value>;
    fn metadata_get(&self, rid: u64) -> ApiFuture<'_, serde_json::Value>;

    fn path_sub_path(&self, path: String, show_hide: bool) -> ApiFuture<'_, Vec<String>>;
    fn initialize(&self, payload: InitializeConfig) -> ApiFuture<'_, InitStatus>;

    fn user_upload_avatar(&self, uid: u64, data: Vec<u8>) -> ApiFuture<'_, PushResponse>;
    fn item_push(&self, data: Vec<u8>) -> ApiFuture<'_, PushResponse>;
    fn item_pull(&self, id: u64) -> ApiFuture<'_, Vec<u8>>;
    fn search(&self, query: SearchRequest) -> ApiFuture<'_, serde_json::Value>;
    fn openapi(&self) -> ApiFuture<'_, String>;
    fn docs(&self) -> ApiFuture<'_, String>;
}
