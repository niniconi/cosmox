pub mod init;
pub mod library;
pub mod metadata;
pub mod path_tree;
pub mod plugin;
pub mod resource;
pub mod role_permission;
pub mod scanner;
pub mod search;
pub mod system;
pub mod tag;
pub mod ui;
pub mod user;

#[derive(Debug, Clone, Default)]
pub enum Endpoint {
    Init,
    Search,

    // User
    GetUser {
        uid: u64,
    },
    DeleteUser {
        uid: u64,
    },
    QueryUser,
    AddRoleForUser {
        rid: u64,
        uid: u64,
    },
    Login,
    Register,
    UploadAvatar {
        uid: u64,
    },

    // Role
    GetRole {
        rid: u64,
    },
    AddRole,
    DeleteRole {
        rid: u64,
    },
    QueryRole,
    GetRolesByUser {
        uid: u64,
    },

    // Permission
    GetPermission {
        pid: u64,
    },
    AddPermission,
    DeletePermission {
        pid: u64,
    },
    QueryPermission,
    GetPermissionsByRole {
        rid: u64,
    },
    GetPermissionsByUser {
        uid: u64,
    },
    AddPermissionForRole {
        pid: u64,
        rid: u64,
    },

    // System
    GetSystemLog,
    GetSystemInfo,
    GetSystemAbout,
    SystemShutdown,
    SystemRestart,
    SystemDeleteAll,

    // Plugin
    InstallPlugin,
    UninstallPlugin,
    EnablePlugin,
    DisablePlugin,
    PluginInfo,
    QueryPlugin,

    // Metadata
    GetMetadata {
        rid: u64,
    },
    QueryMetadata,

    // Library
    AddLibrary,
    GetLibrary {
        lid: u64,
    },
    ModifyLibrary {
        lid: u64,
    },
    DeleteLibrary {
        lid: u64,
    },
    QueryLibrary,
    GetAllLibraryTypes,

    // Tag
    GetTag {
        tid: u64,
    },
    GetTagGroup {
        tgid: u64,
    },
    QueryTag,
    QueryTagGroup,
    GetTagCatalog,
    AddTag,
    AddTagGroup,
    DeleteTag {
        tid: u64,
    },
    DeleteTagGroup {
        tgid: u64,
    },

    // Resource
    GetResource {
        rid: u64,
    },
    AddResource,
    DeleteResource {
        rid: u64,
    },
    QueryResource,
    AddTagForResource {
        rid: u64,
    },
    GetMetadataOfResource {
        rid: u64,
    },

    // Scanner
    Scan {
        lid: u64,
    },
    ScanAll,
    AddScanTask,
    GetProcessOfScan,
    GetScannerInfo,
    GetSubPath,

    // Media transfer
    ItemPush,
    ItemPull {
        pmid: u64,
    },

    // Auth/Static
    Static,
    #[default]
    None,
}
