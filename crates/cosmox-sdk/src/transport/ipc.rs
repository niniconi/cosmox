use std::pin::Pin;

use crate::{
    Api,
    error::SdkError,
    types::{
        InitStatus, InitializeConfig, InstallPlugin, LibrariesRelatedTags, Library, LibraryAdd,
        LibraryDeleteRequest, LibraryModify, LibraryPath, LibraryQueryRequest, LibraryType,
        Permission, PermissionAddRequest, PushResponse, Resource, ResourceAddRequest,
        ResourceModifyRequest, ResourceQueryRequest, Role, RoleAddRequest,
        RoleLinkPermissionAddRequest, ScannerInfo, ScannerStatus, ScannerTaskAddRequest,
        SearchRequest, SystemInfo, Tag, TagAddRequest, TagCatalogEntry, TagGroup,
        TagGroupAddRequest, TagGroupDeleteRequest, TagGroupQueryRequest, TagQueryRequest, User,
        UserLogin, UserQueryRequest, UserResp, UserRoleAddRequest, UserSignUp,
    },
};

pub struct IpcApi;

impl Api for IpcApi {
    fn new(_hostname: &'static str, _port: u16) -> Self {
        Self
    }

    fn set_token(&mut self, _token: String) {}

    fn get_token(&self) -> Option<String> {
        None
    }

    fn logout(&mut self) {}

    fn login(
        &mut self,
        _payload: UserLogin,
    ) -> Pin<Box<dyn Future<Output = Result<(), SdkError>> + Send + '_>> {
        Box::pin(async {
            Err(SdkError::Internal(
                "IPC transport not implemented yet".into(),
            ))
        })
    }

    fn system_info(
        &self,
    ) -> Pin<Box<dyn Future<Output = Result<SystemInfo, SdkError>> + Send + '_>> {
        Box::pin(async {
            Err(SdkError::Internal(
                "IPC transport not implemented yet".into(),
            ))
        })
    }

    fn system_about(&self) -> Pin<Box<dyn Future<Output = Result<String, SdkError>> + Send + '_>> {
        Box::pin(async {
            Err(SdkError::Internal(
                "IPC transport not implemented yet".into(),
            ))
        })
    }

    fn system_log(&self) -> Pin<Box<dyn Future<Output = Result<String, SdkError>> + Send + '_>> {
        Box::pin(async {
            Err(SdkError::Internal(
                "IPC transport not implemented yet".into(),
            ))
        })
    }

    fn system_restart(&self) -> Pin<Box<dyn Future<Output = Result<(), SdkError>> + Send + '_>> {
        Box::pin(async {
            Err(SdkError::Internal(
                "IPC transport not implemented yet".into(),
            ))
        })
    }

    fn system_shutdown(&self) -> Pin<Box<dyn Future<Output = Result<(), SdkError>> + Send + '_>> {
        Box::pin(async {
            Err(SdkError::Internal(
                "IPC transport not implemented yet".into(),
            ))
        })
    }

    fn system_delete_all(&self) -> Pin<Box<dyn Future<Output = Result<(), SdkError>> + Send + '_>> {
        Box::pin(async {
            Err(SdkError::Internal(
                "IPC transport not implemented yet".into(),
            ))
        })
    }

    fn user_get(
        &self,
        _uid: u64,
    ) -> Pin<Box<dyn Future<Output = Result<User, SdkError>> + Send + '_>> {
        Box::pin(async {
            Err(SdkError::Internal(
                "IPC transport not implemented yet".into(),
            ))
        })
    }

    fn user_query(
        &self,
        _params: UserQueryRequest,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<User>, SdkError>> + Send + '_>> {
        Box::pin(async {
            Err(SdkError::Internal(
                "IPC transport not implemented yet".into(),
            ))
        })
    }

    fn user_delete(
        &self,
        _uid: u64,
    ) -> Pin<Box<dyn Future<Output = Result<(), SdkError>> + Send + '_>> {
        Box::pin(async {
            Err(SdkError::Internal(
                "IPC transport not implemented yet".into(),
            ))
        })
    }

    fn user_role_add(
        &self,
        _payload: UserRoleAddRequest,
    ) -> Pin<Box<dyn Future<Output = Result<(), SdkError>> + Send + '_>> {
        Box::pin(async {
            Err(SdkError::Internal(
                "IPC transport not implemented yet".into(),
            ))
        })
    }

    fn library_get(
        &self,
        _lid: u64,
    ) -> Pin<Box<dyn Future<Output = Result<Library, SdkError>> + Send + '_>> {
        Box::pin(async {
            Err(SdkError::Internal(
                "IPC transport not implemented yet".into(),
            ))
        })
    }

    fn library_query(
        &self,
        _params: LibraryQueryRequest,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<Library>, SdkError>> + Send + '_>> {
        Box::pin(async {
            Err(SdkError::Internal(
                "IPC transport not implemented yet".into(),
            ))
        })
    }

    fn library_add(
        &self,
        _payload: LibraryAdd,
    ) -> Pin<
        Box<
            dyn Future<
                    Output = Result<
                        (Library, Vec<LibrariesRelatedTags>, Vec<LibraryPath>),
                        SdkError,
                    >,
                > + Send
                + '_,
        >,
    > {
        Box::pin(async {
            Err(SdkError::Internal(
                "IPC transport not implemented yet".into(),
            ))
        })
    }

    fn library_modify(
        &self,
        _lid: u64,
        _payload: LibraryModify,
    ) -> Pin<Box<dyn Future<Output = Result<(), SdkError>> + Send + '_>> {
        Box::pin(async {
            Err(SdkError::Internal(
                "IPC transport not implemented yet".into(),
            ))
        })
    }

    fn library_delete(
        &self,
        _payload: LibraryDeleteRequest,
    ) -> Pin<Box<dyn Future<Output = Result<(), SdkError>> + Send + '_>> {
        Box::pin(async {
            Err(SdkError::Internal(
                "IPC transport not implemented yet".into(),
            ))
        })
    }

    fn library_type_all(
        &self,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<LibraryType>, SdkError>> + Send + '_>> {
        Box::pin(async {
            Err(SdkError::Internal(
                "IPC transport not implemented yet".into(),
            ))
        })
    }

    fn tag_get(
        &self,
        _tid: u64,
    ) -> Pin<Box<dyn Future<Output = Result<Tag, SdkError>> + Send + '_>> {
        Box::pin(async {
            Err(SdkError::Internal(
                "IPC transport not implemented yet".into(),
            ))
        })
    }

    fn tag_add(
        &self,
        _payload: TagAddRequest,
    ) -> Pin<Box<dyn Future<Output = Result<u64, SdkError>> + Send + '_>> {
        Box::pin(async {
            Err(SdkError::Internal(
                "IPC transport not implemented yet".into(),
            ))
        })
    }

    fn tag_query(
        &self,
        _params: TagQueryRequest,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<Tag>, SdkError>> + Send + '_>> {
        Box::pin(async {
            Err(SdkError::Internal(
                "IPC transport not implemented yet".into(),
            ))
        })
    }

    fn tag_group_get(
        &self,
        _tgid: u64,
    ) -> Pin<Box<dyn Future<Output = Result<TagGroup, SdkError>> + Send + '_>> {
        Box::pin(async {
            Err(SdkError::Internal(
                "IPC transport not implemented yet".into(),
            ))
        })
    }

    fn tag_group_add(
        &self,
        _payload: TagGroupAddRequest,
    ) -> Pin<Box<dyn Future<Output = Result<u64, SdkError>> + Send + '_>> {
        Box::pin(async {
            Err(SdkError::Internal(
                "IPC transport not implemented yet".into(),
            ))
        })
    }

    fn tag_group_delete(
        &self,
        _payload: TagGroupDeleteRequest,
    ) -> Pin<Box<dyn Future<Output = Result<(), SdkError>> + Send + '_>> {
        Box::pin(async {
            Err(SdkError::Internal(
                "IPC transport not implemented yet".into(),
            ))
        })
    }

    fn tag_group_query(
        &self,
        _params: TagGroupQueryRequest,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<TagGroup>, SdkError>> + Send + '_>> {
        Box::pin(async {
            Err(SdkError::Internal(
                "IPC transport not implemented yet".into(),
            ))
        })
    }

    fn tag_catalog(
        &self,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<TagCatalogEntry>, SdkError>> + Send + '_>> {
        Box::pin(async {
            Err(SdkError::Internal(
                "IPC transport not implemented yet".into(),
            ))
        })
    }

    fn resource_get(
        &self,
        _rid: u64,
    ) -> Pin<Box<dyn Future<Output = Result<Resource, SdkError>> + Send + '_>> {
        Box::pin(async {
            Err(SdkError::Internal(
                "IPC transport not implemented yet".into(),
            ))
        })
    }

    fn resource_query(
        &self,
        _params: ResourceQueryRequest,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<Resource>, SdkError>> + Send + '_>> {
        Box::pin(async {
            Err(SdkError::Internal(
                "IPC transport not implemented yet".into(),
            ))
        })
    }

    fn resource_add(
        &self,
        _payload: ResourceAddRequest,
    ) -> Pin<Box<dyn Future<Output = Result<u64, SdkError>> + Send + '_>> {
        Box::pin(async {
            Err(SdkError::Internal(
                "IPC transport not implemented yet".into(),
            ))
        })
    }

    fn resource_delete(
        &self,
        _rid: u64,
    ) -> Pin<Box<dyn Future<Output = Result<(), SdkError>> + Send + '_>> {
        Box::pin(async {
            Err(SdkError::Internal(
                "IPC transport not implemented yet".into(),
            ))
        })
    }

    fn resource_add_tag(
        &self,
        _rid: u64,
        _tag_ids: Vec<u64>,
    ) -> Pin<Box<dyn Future<Output = Result<serde_json::Value, SdkError>> + Send + '_>> {
        Box::pin(async {
            Err(SdkError::Internal(
                "IPC transport not implemented yet".into(),
            ))
        })
    }

    fn resource_get_metadata(
        &self,
        _rid: u64,
    ) -> Pin<Box<dyn Future<Output = Result<serde_json::Value, SdkError>> + Send + '_>> {
        Box::pin(async {
            Err(SdkError::Internal(
                "IPC transport not implemented yet".into(),
            ))
        })
    }

    fn acl_query_role(
        &self,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<Role>, SdkError>> + Send + '_>> {
        Box::pin(async {
            Err(SdkError::Internal(
                "IPC transport not implemented yet".into(),
            ))
        })
    }

    fn acl_query_permission(
        &self,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<Permission>, SdkError>> + Send + '_>> {
        Box::pin(async {
            Err(SdkError::Internal(
                "IPC transport not implemented yet".into(),
            ))
        })
    }

    fn acl_add_role(
        &self,
        _payload: RoleAddRequest,
    ) -> Pin<Box<dyn Future<Output = Result<(), SdkError>> + Send + '_>> {
        Box::pin(async {
            Err(SdkError::Internal(
                "IPC transport not implemented yet".into(),
            ))
        })
    }

    fn acl_delete_role(
        &self,
        _rid: u64,
    ) -> Pin<Box<dyn Future<Output = Result<(), SdkError>> + Send + '_>> {
        Box::pin(async {
            Err(SdkError::Internal(
                "IPC transport not implemented yet".into(),
            ))
        })
    }

    fn acl_add_permission(
        &self,
        _payload: PermissionAddRequest,
    ) -> Pin<Box<dyn Future<Output = Result<(), SdkError>> + Send + '_>> {
        Box::pin(async {
            Err(SdkError::Internal(
                "IPC transport not implemented yet".into(),
            ))
        })
    }

    fn acl_delete_permission(
        &self,
        _pid: u64,
    ) -> Pin<Box<dyn Future<Output = Result<(), SdkError>> + Send + '_>> {
        Box::pin(async {
            Err(SdkError::Internal(
                "IPC transport not implemented yet".into(),
            ))
        })
    }

    fn acl_add_permission_for_role(
        &self,
        _payload: RoleLinkPermissionAddRequest,
    ) -> Pin<Box<dyn Future<Output = Result<(), SdkError>> + Send + '_>> {
        Box::pin(async {
            Err(SdkError::Internal(
                "IPC transport not implemented yet".into(),
            ))
        })
    }

    fn plugin_info(&self) -> Pin<Box<dyn Future<Output = Result<String, SdkError>> + Send + '_>> {
        Box::pin(async {
            Err(SdkError::Internal(
                "IPC transport not implemented yet".into(),
            ))
        })
    }

    fn plugin_install(
        &self,
        _payload: InstallPlugin,
    ) -> Pin<Box<dyn Future<Output = Result<String, SdkError>> + Send + '_>> {
        Box::pin(async {
            Err(SdkError::Internal(
                "IPC transport not implemented yet".into(),
            ))
        })
    }

    fn plugin_uninstall(
        &self,
        _id: u64,
    ) -> Pin<Box<dyn Future<Output = Result<(), SdkError>> + Send + '_>> {
        Box::pin(async {
            Err(SdkError::Internal(
                "IPC transport not implemented yet".into(),
            ))
        })
    }

    fn plugin_enable(
        &self,
        _id: u64,
    ) -> Pin<Box<dyn Future<Output = Result<(), SdkError>> + Send + '_>> {
        Box::pin(async {
            Err(SdkError::Internal(
                "IPC transport not implemented yet".into(),
            ))
        })
    }

    fn plugin_disable(
        &self,
        _id: u64,
    ) -> Pin<Box<dyn Future<Output = Result<(), SdkError>> + Send + '_>> {
        Box::pin(async {
            Err(SdkError::Internal(
                "IPC transport not implemented yet".into(),
            ))
        })
    }

    fn scanner_scan(
        &self,
        _lid: u64,
    ) -> Pin<Box<dyn Future<Output = Result<String, SdkError>> + Send + '_>> {
        Box::pin(async {
            Err(SdkError::Internal(
                "IPC transport not implemented yet".into(),
            ))
        })
    }

    fn scanner_scan_all(
        &self,
    ) -> Pin<Box<dyn Future<Output = Result<String, SdkError>> + Send + '_>> {
        Box::pin(async {
            Err(SdkError::Internal(
                "IPC transport not implemented yet".into(),
            ))
        })
    }

    fn scanner_get_status(
        &self,
    ) -> Pin<Box<dyn Future<Output = Result<ScannerStatus, SdkError>> + Send + '_>> {
        Box::pin(async {
            Err(SdkError::Internal(
                "IPC transport not implemented yet".into(),
            ))
        })
    }

    fn scanner_info(
        &self,
    ) -> Pin<Box<dyn Future<Output = Result<ScannerInfo, SdkError>> + Send + '_>> {
        Box::pin(async {
            Err(SdkError::Internal(
                "IPC transport not implemented yet".into(),
            ))
        })
    }

    fn scanner_add_task(
        &self,
        _payload: ScannerTaskAddRequest,
    ) -> Pin<Box<dyn Future<Output = Result<(), SdkError>> + Send + '_>> {
        Box::pin(async {
            Err(SdkError::Internal(
                "IPC transport not implemented yet".into(),
            ))
        })
    }

    fn metadata_query(
        &self,
        _root_node: u64,
        _depth: usize,
    ) -> Pin<Box<dyn Future<Output = Result<serde_json::Value, SdkError>> + Send + '_>> {
        Box::pin(async {
            Err(SdkError::Internal(
                "IPC transport not implemented yet".into(),
            ))
        })
    }

    fn metadata_get(
        &self,
        _rid: u64,
    ) -> Pin<Box<dyn Future<Output = Result<serde_json::Value, SdkError>> + Send + '_>> {
        Box::pin(async {
            Err(SdkError::Internal(
                "IPC transport not implemented yet".into(),
            ))
        })
    }

    fn path_sub_path(
        &self,
        _path: String,
        _show_hide: bool,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<String>, SdkError>> + Send + '_>> {
        Box::pin(async {
            Err(SdkError::Internal(
                "IPC transport not implemented yet".into(),
            ))
        })
    }

    fn initialize(
        &self,
        _payload: InitializeConfig,
    ) -> Pin<Box<dyn Future<Output = Result<InitStatus, SdkError>> + Send + '_>> {
        Box::pin(async {
            Err(SdkError::Internal(
                "IPC transport not implemented yet".into(),
            ))
        })
    }

    fn search(
        &self,
        _query: SearchRequest,
    ) -> Pin<Box<dyn Future<Output = Result<serde_json::Value, SdkError>> + Send + '_>> {
        Box::pin(async {
            Err(SdkError::Internal(
                "IPC transport not implemented yet".into(),
            ))
        })
    }

    fn openapi(&self) -> Pin<Box<dyn Future<Output = Result<String, SdkError>> + Send + '_>> {
        Box::pin(async {
            Err(SdkError::Internal(
                "IPC transport not implemented yet".into(),
            ))
        })
    }

    fn docs(&self) -> Pin<Box<dyn Future<Output = Result<String, SdkError>> + Send + '_>> {
        Box::pin(async {
            Err(SdkError::Internal(
                "IPC transport not implemented yet".into(),
            ))
        })
    }

    fn user_upload_avatar(
        &self,
        _uid: u64,
        _data: Vec<u8>,
    ) -> Pin<Box<dyn Future<Output = Result<PushResponse, SdkError>> + Send + '_>> {
        Box::pin(async {
            Err(SdkError::Internal(
                "IPC transport not implemented yet".into(),
            ))
        })
    }

    fn resource_modify(
        &self,
        _rid: u64,
        _payload: ResourceModifyRequest,
    ) -> Pin<Box<dyn Future<Output = Result<(), SdkError>> + Send + '_>> {
        Box::pin(async {
            Err(SdkError::Internal(
                "IPC transport not implemented yet".into(),
            ))
        })
    }

    fn user_register(
        &self,
        _payload: UserSignUp,
    ) -> Pin<Box<dyn Future<Output = Result<UserResp, SdkError>> + Send + '_>> {
        Box::pin(async {
            Err(SdkError::Internal(
                "IPC transport not implemented yet".into(),
            ))
        })
    }

    fn item_pull(
        &self,
        _id: u64,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<u8>, SdkError>> + Send + '_>> {
        Box::pin(async {
            Err(SdkError::Internal(
                "IPC transport not implemented yet".into(),
            ))
        })
    }

    fn item_push(
        &self,
        _data: Vec<u8>,
    ) -> Pin<Box<dyn Future<Output = Result<PushResponse, SdkError>> + Send + '_>> {
        Box::pin(async {
            Err(SdkError::Internal(
                "IPC transport not implemented yet".into(),
            ))
        })
    }
}
