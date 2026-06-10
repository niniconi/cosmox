//! IPC view types — entry point for all IPC-serializable types.
//!
//! - `rkyv_ipc_view!` generates View types (Archive + Serialize + From, no Deserialize)
//! - `pub use ... as ...View` re-exports Request types from service crates with a uniform suffix
//!
//! IPC handler code should import everything from here rather than from individual services.

use cosmox_macros::rkyv_ipc_view;

rkyv_ipc_view! {
  pub struct LibraryView for crate::define::Library {
    pub lid: u64,
    pub name: Option<String>,
    pub description: Option<String>,
    pub r#type: Option<u64>,
    pub create_by_uid: u64,
    #[as_i64]
    pub create_datetime: i64,
    #[as_i64]
    pub last_update_datetime: i64,
  }
}

rkyv_ipc_view! {
  pub struct TagView for crate::define::Tag {
    pub tid: u64,
    pub tgid: u64,
    pub text: String,
    #[as_i64]
    pub create_datetime: i64,
  }
}

rkyv_ipc_view! {
  pub struct TagGroupView for crate::define::TagGroups {
    pub tgid: u64,
    pub text: String,
    #[as_i64]
    pub create_datetime: i64,
  }
}

rkyv_ipc_view! {
  pub struct TypeView for crate::define::Type {
    pub tid: u64,
    pub scan_mode: Option<String>,
    pub label: String,
  }
}

rkyv_ipc_view! {
  pub struct ResourceView for crate::define::Resource {
    pub rid: u64,
    pub name: Option<String>,
    pub description: Option<String>,
    pub lid: Option<u64>,
    #[as_i64]
    pub create_datetime: i64,
    #[as_i64]
    pub last_update_datetime: i64,
    pub level: u64,
    pub cover: Option<u64>,
  }
}

rkyv_ipc_view! {
  pub struct UserView for crate::define::User {
    pub uid: u64,
    pub username: String,
    pub avatar: Option<u64>,
    pub nickname: Option<String>,
    #[as_i64]
    pub create_datetime: i64,
    #[as_i64]
    pub last_update_datetime: i64,
    pub email: Option<String>,
    skip password,
  }
}

rkyv_ipc_view! {
  pub struct RoleView for crate::define::Role {
    pub rid: u64,
    pub name: String,
    pub description: Option<String>,
    pub builtin: i8,
  }
}

rkyv_ipc_view! {
  pub struct PermissionView for crate::define::Permission {
    pub pid: u64,
    pub name: String,
    pub description: Option<String>,
    pub builtin: i8,
  }
}

rkyv_ipc_view! {
  pub struct ResourcesRelatedTagsView for crate::define::ResourcesRelatedTags {
    pub rrtid: u64,
    pub rid: u64,
    pub tid: u64,
  }
}

rkyv_ipc_view! {
  pub struct SystemInfoView for crate::services::system_service::SystemInfo {
    pub os: String,
    pub name: String,
    pub version: String,
    pub author: String,
    pub is_first_boot: bool,
  }
}

rkyv_ipc_view! {
  pub struct StatusView for crate::services::init_service::Status {
    pub initialized: bool,
  }
}

rkyv_ipc_view! {
  pub struct UserRespView for crate::services::user_service::UserResp {
    pub uid: u64,
    pub username: String,
    pub email: Option<String>,
  }
}

pub use crate::services::init_service::InitializeConfig as InitializeConfigView;
pub use crate::services::libraries_service::LibraryAddRequest as LibraryAddRequestView;
pub use crate::services::libraries_service::ModifyLibraryRequest as ModifyLibraryRequestView;
pub use crate::services::resource_service::ResourceAddRequest as ResourceAddRequestView;
pub use crate::services::resource_service::ResourceAddTagRequest as ResourceAddTagRequestView;
pub use crate::services::resource_service::ResourceDeleteRequest as ResourceDeleteRequestView;
pub use crate::services::resource_service::ResourceQueryRequest as ResourceQueryRequestView;
pub use crate::services::role_permission_service::PermissionAddRequest as PermissionAddRequestView;
pub use crate::services::role_permission_service::RoleAddRequest as RoleAddRequestView;
pub use crate::services::tag_service::TagAddRequest as TagAddRequestView;
pub use crate::services::tag_service::TagGroupAddRequest as TagGroupAddRequestView;
pub use crate::services::tag_service::TagGroupDeleteRequest as TagGroupDeleteRequestView;
pub use crate::services::tag_service::TagGroupQueryRequest as TagGroupQueryRequestView;
pub use crate::services::tag_service::TagQueryRequest as TagQueryRequestView;
pub use crate::services::user_service::UserQueryRequest as UserQueryRequestView;

#[derive(rkyv::Archive, rkyv::Serialize)]
#[rkyv(bytecheck())]
pub struct TagCatalogEntryView {
    pub group: TagGroupView,
    pub tags: Vec<TagView>,
}

impl From<crate::services::tag_service::TagCatalogEntry> for TagCatalogEntryView {
    fn from(v: crate::services::tag_service::TagCatalogEntry) -> Self {
        let crate::services::tag_service::TagCatalogEntry { group, tags } = v;
        Self {
            group: TagGroupView::from(group),
            tags: tags.into_iter().map(TagView::from).collect(),
        }
    }
}
