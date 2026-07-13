use crate::{
    Api, ApiFuture,
    error::SdkError,
    types::{
        InitStatus, InitializeConfig, InstallPlugin, LibrariesRelatedTags, Library, LibraryAdd,
        LibraryDeleteRequest, LibraryModify, LibraryPath, LibraryQueryRequest, LibraryType,
        Permission, PermissionAddRequest, PluginQueryItem, PluginQueryRequest, PushResponse,
        Resource, ResourceAddRequest, ResourceModifyRequest, ResourceQueryRequest, Role,
        RoleAddRequest, RoleLinkPermissionAddRequest, ScannerInfo, ScannerStatus,
        ScannerTaskAddRequest, SearchRequest, SystemInfo, Tag, TagAddRequest, TagCatalogEntry,
        TagGroup, TagGroupAddRequest, TagGroupDeleteRequest, TagGroupQueryRequest, TagQueryRequest,
        User, UserLogin, UserQueryRequest, UserResp, UserRoleAddRequest, UserSignUp,
    },
};

pub struct DirectApi;

impl Api for DirectApi {
    fn new(_hostname: &'static str, _port: u16) -> Self {
        Self
    }

    fn set_token(&mut self, _token: String) {}

    fn get_token(&self) -> Option<String> {
        None
    }

    fn logout(&mut self) {}

    fn login(&mut self, _payload: UserLogin) -> ApiFuture<'_, ()> {
        Box::pin(async {
            Err(SdkError::Internal(
                "Direct transport not implemented yet".into(),
            ))
        })
    }

    fn system_info(&self) -> ApiFuture<'_, SystemInfo> {
        Box::pin(async {
            Err(SdkError::Internal(
                "Direct transport not implemented yet".into(),
            ))
        })
    }
    fn system_about(&self) -> ApiFuture<'_, String> {
        Box::pin(async {
            Err(SdkError::Internal(
                "Direct transport not implemented yet".into(),
            ))
        })
    }
    fn system_log(&self) -> ApiFuture<'_, String> {
        Box::pin(async {
            Err(SdkError::Internal(
                "Direct transport not implemented yet".into(),
            ))
        })
    }
    fn system_restart(&self) -> ApiFuture<'_, ()> {
        Box::pin(async {
            Err(SdkError::Internal(
                "Direct transport not implemented yet".into(),
            ))
        })
    }
    fn system_shutdown(&self) -> ApiFuture<'_, ()> {
        Box::pin(async {
            Err(SdkError::Internal(
                "Direct transport not implemented yet".into(),
            ))
        })
    }
    fn system_delete_all(&self) -> ApiFuture<'_, ()> {
        Box::pin(async {
            Err(SdkError::Internal(
                "Direct transport not implemented yet".into(),
            ))
        })
    }

    fn user_get(&self, _uid: u64) -> ApiFuture<'_, User> {
        Box::pin(async {
            Err(SdkError::Internal(
                "Direct transport not implemented yet".into(),
            ))
        })
    }
    fn user_query(&self, _params: UserQueryRequest) -> ApiFuture<'_, Vec<User>> {
        Box::pin(async {
            Err(SdkError::Internal(
                "Direct transport not implemented yet".into(),
            ))
        })
    }

    fn user_delete(&self, _uid: u64) -> ApiFuture<'_, ()> {
        Box::pin(async {
            Err(SdkError::Internal(
                "Direct transport not implemented yet".into(),
            ))
        })
    }
    fn user_role_add(&self, _payload: UserRoleAddRequest) -> ApiFuture<'_, ()> {
        Box::pin(async {
            Err(SdkError::Internal(
                "Direct transport not implemented yet".into(),
            ))
        })
    }

    fn library_get(&self, _lid: u64) -> ApiFuture<'_, Library> {
        Box::pin(async {
            Err(SdkError::Internal(
                "Direct transport not implemented yet".into(),
            ))
        })
    }

    fn library_query(&self, _params: LibraryQueryRequest) -> ApiFuture<'_, Vec<Library>> {
        Box::pin(async {
            Err(SdkError::Internal(
                "Direct transport not implemented yet".into(),
            ))
        })
    }

    fn library_add(
        &self,
        _payload: LibraryAdd,
    ) -> ApiFuture<'_, (Library, Vec<LibrariesRelatedTags>, Vec<LibraryPath>)> {
        Box::pin(async {
            Err(SdkError::Internal(
                "Direct transport not implemented yet".into(),
            ))
        })
    }
    fn library_modify(&self, _lid: u64, _payload: LibraryModify) -> ApiFuture<'_, ()> {
        Box::pin(async {
            Err(SdkError::Internal(
                "Direct transport not implemented yet".into(),
            ))
        })
    }
    fn library_delete(&self, _payload: LibraryDeleteRequest) -> ApiFuture<'_, ()> {
        Box::pin(async {
            Err(SdkError::Internal(
                "Direct transport not implemented yet".into(),
            ))
        })
    }
    fn library_type_all(&self) -> ApiFuture<'_, Vec<LibraryType>> {
        Box::pin(async {
            Err(SdkError::Internal(
                "Direct transport not implemented yet".into(),
            ))
        })
    }

    fn tag_get(&self, _tid: u64) -> ApiFuture<'_, Tag> {
        Box::pin(async {
            Err(SdkError::Internal(
                "Direct transport not implemented yet".into(),
            ))
        })
    }
    fn tag_add(&self, _payload: TagAddRequest) -> ApiFuture<'_, u64> {
        Box::pin(async {
            Err(SdkError::Internal(
                "Direct transport not implemented yet".into(),
            ))
        })
    }
    fn tag_query(&self, _params: TagQueryRequest) -> ApiFuture<'_, Vec<Tag>> {
        Box::pin(async {
            Err(SdkError::Internal(
                "Direct transport not implemented yet".into(),
            ))
        })
    }

    fn tag_group_get(&self, _tgid: u64) -> ApiFuture<'_, TagGroup> {
        Box::pin(async {
            Err(SdkError::Internal(
                "Direct transport not implemented yet".into(),
            ))
        })
    }
    fn tag_group_add(&self, _payload: TagGroupAddRequest) -> ApiFuture<'_, u64> {
        Box::pin(async {
            Err(SdkError::Internal(
                "Direct transport not implemented yet".into(),
            ))
        })
    }
    fn tag_group_delete(&self, _payload: TagGroupDeleteRequest) -> ApiFuture<'_, ()> {
        Box::pin(async {
            Err(SdkError::Internal(
                "Direct transport not implemented yet".into(),
            ))
        })
    }
    fn tag_group_query(&self, _params: TagGroupQueryRequest) -> ApiFuture<'_, Vec<TagGroup>> {
        Box::pin(async {
            Err(SdkError::Internal(
                "Direct transport not implemented yet".into(),
            ))
        })
    }
    fn tag_catalog(&self) -> ApiFuture<'_, Vec<TagCatalogEntry>> {
        Box::pin(async {
            Err(SdkError::Internal(
                "Direct transport not implemented yet".into(),
            ))
        })
    }

    fn resource_get(&self, _rid: u64) -> ApiFuture<'_, Resource> {
        Box::pin(async {
            Err(SdkError::Internal(
                "Direct transport not implemented yet".into(),
            ))
        })
    }
    fn resource_query(&self, _params: ResourceQueryRequest) -> ApiFuture<'_, Vec<Resource>> {
        Box::pin(async {
            Err(SdkError::Internal(
                "Direct transport not implemented yet".into(),
            ))
        })
    }
    fn resource_add(&self, _payload: ResourceAddRequest) -> ApiFuture<'_, u64> {
        Box::pin(async {
            Err(SdkError::Internal(
                "Direct transport not implemented yet".into(),
            ))
        })
    }
    fn resource_delete(&self, _rid: u64) -> ApiFuture<'_, ()> {
        Box::pin(async {
            Err(SdkError::Internal(
                "Direct transport not implemented yet".into(),
            ))
        })
    }
    fn resource_add_tag(&self, _rid: u64, _tag_ids: Vec<u64>) -> ApiFuture<'_, serde_json::Value> {
        Box::pin(async {
            Err(SdkError::Internal(
                "Direct transport not implemented yet".into(),
            ))
        })
    }
    fn resource_get_metadata(&self, _rid: u64) -> ApiFuture<'_, serde_json::Value> {
        Box::pin(async {
            Err(SdkError::Internal(
                "Direct transport not implemented yet".into(),
            ))
        })
    }

    fn acl_query_role(&self) -> ApiFuture<'_, Vec<Role>> {
        Box::pin(async {
            Err(SdkError::Internal(
                "Direct transport not implemented yet".into(),
            ))
        })
    }
    fn acl_query_permission(&self) -> ApiFuture<'_, Vec<Permission>> {
        Box::pin(async {
            Err(SdkError::Internal(
                "Direct transport not implemented yet".into(),
            ))
        })
    }
    fn acl_add_role(&self, _payload: RoleAddRequest) -> ApiFuture<'_, ()> {
        Box::pin(async {
            Err(SdkError::Internal(
                "Direct transport not implemented yet".into(),
            ))
        })
    }
    fn acl_delete_role(&self, _rid: u64) -> ApiFuture<'_, ()> {
        Box::pin(async {
            Err(SdkError::Internal(
                "Direct transport not implemented yet".into(),
            ))
        })
    }
    fn acl_add_permission(&self, _payload: PermissionAddRequest) -> ApiFuture<'_, ()> {
        Box::pin(async {
            Err(SdkError::Internal(
                "Direct transport not implemented yet".into(),
            ))
        })
    }
    fn acl_delete_permission(&self, _pid: u64) -> ApiFuture<'_, ()> {
        Box::pin(async {
            Err(SdkError::Internal(
                "Direct transport not implemented yet".into(),
            ))
        })
    }
    fn acl_add_permission_for_role(
        &self,
        _payload: RoleLinkPermissionAddRequest,
    ) -> ApiFuture<'_, ()> {
        Box::pin(async {
            Err(SdkError::Internal(
                "Direct transport not implemented yet".into(),
            ))
        })
    }

    fn plugin_info(&self) -> ApiFuture<'_, String> {
        Box::pin(async {
            Err(SdkError::Internal(
                "Direct transport not implemented yet".into(),
            ))
        })
    }

    fn plugin_query(&self, _params: PluginQueryRequest) -> ApiFuture<'_, Vec<PluginQueryItem>> {
        Box::pin(async {
            Err(SdkError::Internal(
                "Direct transport not implemented yet".into(),
            ))
        })
    }

    fn plugin_install(&self, _payload: InstallPlugin) -> ApiFuture<'_, String> {
        Box::pin(async {
            Err(SdkError::Internal(
                "Direct transport not implemented yet".into(),
            ))
        })
    }
    fn plugin_uninstall(&self, _name: String) -> ApiFuture<'_, ()> {
        Box::pin(async {
            Err(SdkError::Internal(
                "Direct transport not implemented yet".into(),
            ))
        })
    }
    fn plugin_enable(&self, _name: String) -> ApiFuture<'_, ()> {
        Box::pin(async {
            Err(SdkError::Internal(
                "Direct transport not implemented yet".into(),
            ))
        })
    }
    fn plugin_disable(&self, _name: String) -> ApiFuture<'_, ()> {
        Box::pin(async {
            Err(SdkError::Internal(
                "Direct transport not implemented yet".into(),
            ))
        })
    }

    fn scanner_scan(&self, _lid: u64) -> ApiFuture<'_, String> {
        Box::pin(async {
            Err(SdkError::Internal(
                "Direct transport not implemented yet".into(),
            ))
        })
    }
    fn scanner_scan_all(&self) -> ApiFuture<'_, String> {
        Box::pin(async {
            Err(SdkError::Internal(
                "Direct transport not implemented yet".into(),
            ))
        })
    }
    fn scanner_get_status(&self) -> ApiFuture<'_, ScannerStatus> {
        Box::pin(async {
            Err(SdkError::Internal(
                "Direct transport not implemented yet".into(),
            ))
        })
    }
    fn scanner_info(&self) -> ApiFuture<'_, ScannerInfo> {
        Box::pin(async {
            Err(SdkError::Internal(
                "Direct transport not implemented yet".into(),
            ))
        })
    }
    fn scanner_add_task(&self, _payload: ScannerTaskAddRequest) -> ApiFuture<'_, ()> {
        Box::pin(async {
            Err(SdkError::Internal(
                "Direct transport not implemented yet".into(),
            ))
        })
    }

    fn metadata_query(&self, _root_node: u64, _depth: usize) -> ApiFuture<'_, serde_json::Value> {
        Box::pin(async {
            Err(SdkError::Internal(
                "Direct transport not implemented yet".into(),
            ))
        })
    }
    fn metadata_get(&self, _rid: u64) -> ApiFuture<'_, serde_json::Value> {
        Box::pin(async {
            Err(SdkError::Internal(
                "Direct transport not implemented yet".into(),
            ))
        })
    }

    fn path_sub_path(&self, _path: String, _show_hide: bool) -> ApiFuture<'_, Vec<String>> {
        Box::pin(async {
            Err(SdkError::Internal(
                "Direct transport not implemented yet".into(),
            ))
        })
    }

    fn initialize(&self, _payload: InitializeConfig) -> ApiFuture<'_, InitStatus> {
        Box::pin(async {
            Err(SdkError::Internal(
                "Direct transport not implemented yet".into(),
            ))
        })
    }

    fn search(&self, _query: SearchRequest) -> ApiFuture<'_, serde_json::Value> {
        Box::pin(async {
            Err(SdkError::Internal(
                "Direct transport not implemented yet".into(),
            ))
        })
    }

    fn openapi(&self) -> ApiFuture<'_, String> {
        Box::pin(async {
            Err(SdkError::Internal(
                "Direct transport not implemented yet".into(),
            ))
        })
    }
    fn docs(&self) -> ApiFuture<'_, String> {
        Box::pin(async {
            Err(SdkError::Internal(
                "Direct transport not implemented yet".into(),
            ))
        })
    }

    fn item_push(&self, _data: Vec<u8>) -> ApiFuture<'_, PushResponse> {
        Box::pin(async {
            Err(SdkError::Internal(
                "Direct transport not implemented yet".into(),
            ))
        })
    }

    fn item_pull(&self, _id: u64) -> ApiFuture<'_, Vec<u8>> {
        Box::pin(async {
            Err(SdkError::Internal(
                "Direct transport not implemented yet".into(),
            ))
        })
    }
    fn user_register(&self, _payload: UserSignUp) -> ApiFuture<'_, UserResp> {
        Box::pin(async {
            Err(SdkError::Internal(
                "Direct transport not implemented yet".into(),
            ))
        })
    }
    fn resource_modify(&self, _rid: u64, _payload: ResourceModifyRequest) -> ApiFuture<'_, ()> {
        Box::pin(async {
            Err(SdkError::Internal(
                "Direct transport not implemented yet".into(),
            ))
        })
    }

    fn user_upload_avatar(&self, _uid: u64, _data: Vec<u8>) -> ApiFuture<'_, PushResponse> {
        Box::pin(async {
            Err(SdkError::Internal(
                "Direct transport not implemented yet".into(),
            ))
        })
    }
}
