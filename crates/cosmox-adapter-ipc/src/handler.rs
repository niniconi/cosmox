//! IPC endpoint handlers — one async fn per endpoint.
use cosmox_backend_api::{
    Context, Token,
    api::{self, system},
    message::MessagePayload,
};
use cosmox_backend_data::{ipc_views::*, services::search_service::SearchRequest};

use rkyv::rancor;

use crate::protocol::*;

/// Call an API that returns data, with `$bind` bound to the payload.
macro_rules! handle_api {
    ($api_call:expr, $bind:ident, $body:expr) => {{
        match $api_call.await {
            Ok(msg) => match msg.payload {
                Some(MessagePayload::Data($bind)) => ok_response!(&$body),
                _ => IpcResponse::error("Empty payload"),
            },
            Err(e) => IpcResponse::error(e.to_string()),
        }
    }};
}

macro_rules! handle_api_void {
    ($api_call:expr) => {{
        match $api_call.await {
            Ok(_) => IpcResponse::ok(vec![]),
            Err(e) => IpcResponse::error(e.to_string()),
        }
    }};
}

macro_rules! ok_response {
    ($data:expr) => {
        match rkyv::to_bytes::<rancor::Error>($data) {
            Ok(aligned) => IpcResponse::ok(aligned.into_vec()),
            Err(e) => IpcResponse::error(e.to_string()),
        }
    };
}

fn make_context(token: Option<String>) -> Context<'static> {
    Context::builder().token(Token(token)).build()
}

/// Deserialize a request payload from rkyv bytes.
fn rkyv_payload<T>(payload: &[u8]) -> Result<T, String>
where
    T: rkyv::Archive,
    T::Archived: for<'a> rkyv::bytecheck::CheckBytes<rkyv::api::high::HighValidator<'a, rkyv::rancor::Error>>
        + rkyv::Deserialize<T, rkyv::rancor::Strategy<rkyv::de::Pool, rkyv::rancor::Error>>,
{
    rkyv::from_bytes::<T, rkyv::rancor::Error>(payload).map_err(|e| format!("Invalid payload: {e}"))
}

fn json_payload<T: serde::de::DeserializeOwned>(payload: &[u8]) -> Result<T, String> {
    serde_json::from_slice(payload).map_err(|e| format!("Invalid payload (json): {e}"))
}

pub(super) async fn handle_system_info(token: Option<String>) -> IpcResponse {
    let mut ctx = make_context(token);
    handle_api!(system::info(&mut ctx), data, SystemInfoView::from(data))
}

pub(super) async fn handle_system_about(token: Option<String>) -> IpcResponse {
    let mut ctx = make_context(token);
    handle_api!(system::about(&mut ctx), data, data)
}

pub(super) async fn handle_system_log(token: Option<String>) -> IpcResponse {
    let mut ctx = make_context(token);
    handle_api!(system::log(&mut ctx), data, data)
}

pub(super) async fn handle_system_shutdown(token: Option<String>) -> IpcResponse {
    let mut ctx = make_context(token);
    handle_api_void!(system::shutdown(&mut ctx))
}

pub(super) async fn handle_get_library(token: Option<String>, lid: u64) -> IpcResponse {
    let mut ctx = make_context(token);
    handle_api!(
        api::library::get(&mut ctx, lid),
        data,
        LibraryView::from(data)
    )
}

pub(super) async fn handle_query_library(token: Option<String>) -> IpcResponse {
    let mut ctx = make_context(token);
    handle_api!(
        api::library::query(
            &mut ctx,
            cosmox_backend_data::services::libraries_service::LibraryQueryRequest {
                sort: None,
                page: Some(0),
                page_size: 20,
            },
        ),
        data,
        data.into_iter().map(LibraryView::from).collect::<Vec<_>>()
    )
}

pub(super) async fn handle_modify_library(
    token: Option<String>,
    lid: u64,
    payload: Vec<u8>,
) -> IpcResponse {
    let req = match rkyv_payload::<ModifyLibraryRequestView>(&payload) {
        Ok(r) => r,
        Err(e) => return IpcResponse::error(e),
    };
    let mut ctx = make_context(token);
    handle_api_void!(api::library::modify(&mut ctx, lid, req))
}

pub(super) async fn handle_add_library(token: Option<String>, payload: Vec<u8>) -> IpcResponse {
    let req = match rkyv_payload::<LibraryAddRequestView>(&payload) {
        Ok(r) => r,
        Err(e) => return IpcResponse::error(e),
    };
    let mut ctx = make_context(token);
    handle_api!(
        api::library::add(&mut ctx, std::sync::Arc::new(req)),
        data,
        LibraryView::from(data.0)
    )
}

pub(super) async fn handle_delete_library(token: Option<String>, lid: u64) -> IpcResponse {
    let mut ctx = make_context(token);
    handle_api_void!(api::library::delete(&mut ctx, lid))
}

pub(super) async fn handle_get_all_library_types(token: Option<String>) -> IpcResponse {
    let mut ctx = make_context(token);
    handle_api!(
        api::library::get_all_type(&mut ctx),
        data,
        data.into_iter().map(TypeView::from).collect::<Vec<_>>()
    )
}

pub(super) async fn handle_get_tag(token: Option<String>, tid: u64) -> IpcResponse {
    let mut ctx = make_context(token);
    handle_api!(api::tag::get(&mut ctx, tid), data, TagView::from(data))
}

pub(super) async fn handle_get_tag_group(token: Option<String>, tgid: u64) -> IpcResponse {
    let mut ctx = make_context(token);
    handle_api!(
        api::tag::get_group(&mut ctx, tgid),
        data,
        TagGroupView::from(data)
    )
}

pub(super) async fn handle_get_tag_catalog(token: Option<String>) -> IpcResponse {
    let mut ctx = make_context(token);
    handle_api!(
        api::tag::catalog(&mut ctx),
        data,
        data.into_iter()
            .map(From::from)
            .collect::<Vec<TagCatalogEntryView>>()
    )
}

pub(super) async fn handle_query_tag(token: Option<String>, payload: Vec<u8>) -> IpcResponse {
    let params = match rkyv_payload::<TagQueryRequestView>(&payload) {
        Ok(r) => r,
        Err(e) => return IpcResponse::error(e),
    };
    let mut ctx = make_context(token);
    handle_api!(
        api::tag::query(&mut ctx, params),
        data,
        data.into_iter().map(TagView::from).collect::<Vec<_>>()
    )
}

pub(super) async fn handle_add_tag(token: Option<String>, payload: Vec<u8>) -> IpcResponse {
    let req = match rkyv_payload::<TagAddRequestView>(&payload) {
        Ok(r) => r,
        Err(e) => return IpcResponse::error(e),
    };
    let mut ctx = make_context(token);
    handle_api!(api::tag::add(&mut ctx, req), data, data)
}

pub(super) async fn handle_query_tag_group(token: Option<String>, payload: Vec<u8>) -> IpcResponse {
    let params = match rkyv_payload::<TagGroupQueryRequestView>(&payload) {
        Ok(r) => r,
        Err(e) => return IpcResponse::error(e),
    };
    let mut ctx = make_context(token);
    handle_api!(
        api::tag::query_group(&mut ctx, params),
        data,
        data.into_iter().map(TagGroupView::from).collect::<Vec<_>>()
    )
}

pub(super) async fn handle_group_add_tag(token: Option<String>, payload: Vec<u8>) -> IpcResponse {
    let req = match rkyv_payload::<TagGroupAddRequestView>(&payload) {
        Ok(r) => r,
        Err(e) => return IpcResponse::error(e),
    };
    let mut ctx = make_context(token);
    handle_api!(api::tag::add_group(&mut ctx, req), data, data)
}

pub(super) async fn handle_delete_tag_group(token: Option<String>, tgid: u64) -> IpcResponse {
    let mut ctx = make_context(token);
    handle_api_void!(api::tag::delete_group(
        &mut ctx,
        TagGroupDeleteRequestView { tgid },
    ))
}

pub(super) async fn handle_get_resource(token: Option<String>, rid: u64) -> IpcResponse {
    let mut ctx = make_context(token);
    handle_api!(
        api::resource::get(&mut ctx, rid),
        data,
        ResourceView::from(data)
    )
}

pub(super) async fn handle_add_resource(token: Option<String>, payload: Vec<u8>) -> IpcResponse {
    let req = match rkyv_payload::<ResourceAddRequestView>(&payload) {
        Ok(r) => r,
        Err(e) => return IpcResponse::error(e),
    };
    let mut ctx = make_context(token);
    handle_api!(api::resource::add(&mut ctx, req), data, data)
}

pub(super) async fn handle_delete_resource(token: Option<String>, rid: u64) -> IpcResponse {
    let mut ctx = make_context(token);
    handle_api_void!(api::resource::delete(
        &mut ctx,
        ResourceDeleteRequestView { rid }
    ))
}

pub(super) async fn handle_query_resource(token: Option<String>, payload: Vec<u8>) -> IpcResponse {
    let params = match rkyv_payload::<ResourceQueryRequestView>(&payload) {
        Ok(r) => r,
        Err(e) => return IpcResponse::error(e),
    };
    let mut ctx = make_context(token);
    handle_api!(
        api::resource::query(&mut ctx, params),
        data,
        data.into_iter().map(ResourceView::from).collect::<Vec<_>>()
    )
}

pub(super) async fn handle_add_tag_for_resource(
    token: Option<String>,
    rid: u64,
    payload: Vec<u8>,
) -> IpcResponse {
    let req = match rkyv_payload::<ResourceAddTagRequestView>(&payload) {
        Ok(r) => r,
        Err(e) => return IpcResponse::error(e),
    };
    let mut ctx = make_context(token);
    handle_api!(
        api::resource::add_tag(&mut ctx, rid, req),
        data,
        data.into_iter()
            .map(ResourcesRelatedTagsView::from)
            .collect::<Vec<_>>()
    )
}

pub(super) async fn handle_get_user(token: Option<String>, uid: u64) -> IpcResponse {
    let mut ctx = make_context(token);
    handle_api!(
        api::user::get_user(&mut ctx, uid),
        data,
        UserView::from(data)
    )
}

pub(super) async fn handle_query_user(token: Option<String>, payload: Vec<u8>) -> IpcResponse {
    let params = match rkyv_payload::<UserQueryRequestView>(&payload) {
        Ok(r) => r,
        Err(e) => return IpcResponse::error(e),
    };
    let mut ctx = make_context(token);
    handle_api!(
        api::user::query(&mut ctx, std::sync::Arc::new(params)),
        data,
        data.into_iter().map(UserView::from).collect::<Vec<_>>()
    )
}

pub(super) async fn handle_scan_library(token: Option<String>, lid: u64) -> IpcResponse {
    let mut ctx = make_context(token);
    handle_api!(api::scanner::scan(&mut ctx, lid), data, data.to_string())
}

pub(super) async fn handle_scan_all(token: Option<String>) -> IpcResponse {
    let mut ctx = make_context(token);
    handle_api!(api::scanner::scan_all(&mut ctx), data, data.to_string())
}

pub(super) async fn handle_search(token: Option<String>, payload: Vec<u8>) -> IpcResponse {
    let params = match json_payload::<SearchRequest>(&payload) {
        Ok(r) => r,
        Err(e) => return IpcResponse::error(e),
    };
    let mut ctx = make_context(token);
    handle_api!(
        api::search::search(&mut ctx, params),
        data,
        data.into_iter().map(ResourceView::from).collect::<Vec<_>>()
    )
}

pub(super) async fn handle_initialize(token: Option<String>, payload: Vec<u8>) -> IpcResponse {
    let config = match rkyv_payload::<InitializeConfigView>(&payload) {
        Ok(r) => r,
        Err(e) => return IpcResponse::error(e),
    };
    let mut ctx = make_context(token);
    handle_api!(
        api::init::initialize(&mut ctx, config),
        data,
        StatusView::from(data)
    )
}

#[derive(serde::Deserialize)]
struct IpcGetSubPathReq {
    path: String,
    show_hide: bool,
}

pub(super) async fn handle_get_sub_path(token: Option<String>, payload: Vec<u8>) -> IpcResponse {
    let req = match json_payload::<IpcGetSubPathReq>(&payload) {
        Ok(r) => r,
        Err(e) => return IpcResponse::error(e),
    };
    let mut ctx = make_context(token);
    handle_api!(
        api::path_tree::get_sub_path(&mut ctx, req.path, req.show_hide),
        data,
        data
    )
}

pub(super) async fn handle_get_role(token: Option<String>, rid: u64) -> IpcResponse {
    let mut ctx = make_context(token);
    handle_api!(
        api::role_permission::get_role(&mut ctx, rid),
        data,
        RoleView::from(data)
    )
}

pub(super) async fn handle_query_role(token: Option<String>) -> IpcResponse {
    let mut ctx = make_context(token);
    handle_api!(
        api::role_permission::query_role(&mut ctx),
        data,
        data.into_iter().map(RoleView::from).collect::<Vec<_>>()
    )
}

pub(super) async fn handle_add_role(token: Option<String>, payload: Vec<u8>) -> IpcResponse {
    let req = match rkyv_payload::<RoleAddRequestView>(&payload) {
        Ok(r) => r,
        Err(e) => return IpcResponse::error(e),
    };
    let mut ctx = make_context(token);
    handle_api_void!(api::role_permission::add_role(&mut ctx, req))
}

pub(super) async fn handle_delete_role(token: Option<String>, rid: u64) -> IpcResponse {
    let mut ctx = make_context(token);
    handle_api_void!(api::role_permission::delete_role(&mut ctx, rid))
}

pub(super) async fn handle_get_roles_by_user(token: Option<String>, uid: u64) -> IpcResponse {
    let mut ctx = make_context(token);
    handle_api!(
        api::role_permission::get_roles_by_user(&mut ctx, uid),
        data,
        data.into_iter().map(RoleView::from).collect::<Vec<_>>()
    )
}

pub(super) async fn handle_get_permission(token: Option<String>, pid: u64) -> IpcResponse {
    let mut ctx = make_context(token);
    handle_api!(
        api::role_permission::get_permission(&mut ctx, pid),
        data,
        PermissionView::from(data)
    )
}

pub(super) async fn handle_query_permission(token: Option<String>) -> IpcResponse {
    let mut ctx = make_context(token);
    handle_api!(
        api::role_permission::query_permission(&mut ctx),
        data,
        data.into_iter()
            .map(PermissionView::from)
            .collect::<Vec<_>>()
    )
}

pub(super) async fn handle_add_permission(token: Option<String>, payload: Vec<u8>) -> IpcResponse {
    let req = match rkyv_payload::<PermissionAddRequestView>(&payload) {
        Ok(r) => r,
        Err(e) => return IpcResponse::error(e),
    };
    let mut ctx = make_context(token);
    handle_api_void!(api::role_permission::add_permission(&mut ctx, req))
}

pub(super) async fn handle_delete_permission(token: Option<String>, pid: u64) -> IpcResponse {
    let mut ctx = make_context(token);
    handle_api_void!(api::role_permission::delete_permission(&mut ctx, pid))
}

pub(super) async fn handle_get_permissions_by_role(token: Option<String>, rid: u64) -> IpcResponse {
    let mut ctx = make_context(token);
    handle_api!(
        api::role_permission::get_permissions_by_role(&mut ctx, rid),
        data,
        data.into_iter()
            .map(PermissionView::from)
            .collect::<Vec<_>>()
    )
}

pub(super) async fn handle_get_permissions_by_user(token: Option<String>, uid: u64) -> IpcResponse {
    let mut ctx = make_context(token);
    handle_api!(
        api::role_permission::get_permissions_by_user(&mut ctx, uid),
        data,
        data.into_iter()
            .map(PermissionView::from)
            .collect::<Vec<_>>()
    )
}

pub(super) async fn handle_add_permission_for_role(
    token: Option<String>,
    pid: u64,
    rid: u64,
) -> IpcResponse {
    let mut ctx = make_context(token);
    handle_api_void!(api::role_permission::add_permission_for_role(
        &mut ctx, pid, rid
    ))
}

pub(super) async fn handle_add_role_for_user(
    token: Option<String>,
    rid: u64,
    uid: u64,
) -> IpcResponse {
    let mut ctx = make_context(token);
    handle_api_void!(api::role_permission::add_role_for_user(&mut ctx, rid, uid))
}

pub(super) async fn handle_plugin_info(token: Option<String>) -> IpcResponse {
    let mut ctx = make_context(token);
    handle_api!(api::plugin::info(&mut ctx), data, data)
}

pub(super) async fn dispatch(request: IpcRequest) -> IpcResponse {
    let IpcRequest {
        endpoint,
        token,
        payload,
    } = request;
    match endpoint {
        IpcEndpoint::GetSystemInfo => handle_system_info(token).await,
        IpcEndpoint::GetSystemAbout => handle_system_about(token).await,
        IpcEndpoint::GetSystemLog => handle_system_log(token).await,
        IpcEndpoint::SystemShutdown => handle_system_shutdown(token).await,
        IpcEndpoint::SystemRestart => IpcResponse::error("Not implemented"),
        IpcEndpoint::GetLibrary(lid) => handle_get_library(token, lid).await,
        IpcEndpoint::QueryLibrary => handle_query_library(token).await,
        IpcEndpoint::ModifyLibrary(lid) => handle_modify_library(token, lid, payload).await,
        IpcEndpoint::AddLibrary => handle_add_library(token, payload).await,
        IpcEndpoint::DeleteLibrary(lid) => handle_delete_library(token, lid).await,
        IpcEndpoint::GetAllLibraryTypes => handle_get_all_library_types(token).await,
        IpcEndpoint::GetTag(tid) => handle_get_tag(token, tid).await,
        IpcEndpoint::GetTagGroup(tgid) => handle_get_tag_group(token, tgid).await,
        IpcEndpoint::GetTagCatalog => handle_get_tag_catalog(token).await,
        IpcEndpoint::QueryTag => handle_query_tag(token, payload).await,
        IpcEndpoint::AddTag => handle_add_tag(token, payload).await,
        IpcEndpoint::QueryTagGroup => handle_query_tag_group(token, payload).await,
        IpcEndpoint::AddTagGroup => handle_group_add_tag(token, payload).await,
        IpcEndpoint::DeleteTagGroup(tgid) => handle_delete_tag_group(token, tgid).await,
        IpcEndpoint::GetResource(rid) => handle_get_resource(token, rid).await,
        IpcEndpoint::AddResource => handle_add_resource(token, payload).await,
        IpcEndpoint::DeleteResource(rid) => handle_delete_resource(token, rid).await,
        IpcEndpoint::QueryResource => handle_query_resource(token, payload).await,
        IpcEndpoint::AddTagForResource(rid) => {
            handle_add_tag_for_resource(token, rid, payload).await
        }
        IpcEndpoint::GetUser(uid) => handle_get_user(token, uid).await,
        IpcEndpoint::QueryUser => handle_query_user(token, payload).await,
        IpcEndpoint::ScanLibrary(lid) => handle_scan_library(token, lid).await,
        IpcEndpoint::ScanAll => handle_scan_all(token).await,
        IpcEndpoint::Search => handle_search(token, payload).await,
        IpcEndpoint::Initialize => handle_initialize(token, payload).await,
        IpcEndpoint::GetSubPath => handle_get_sub_path(token, payload).await,
        IpcEndpoint::GetRole(rid) => handle_get_role(token, rid).await,
        IpcEndpoint::QueryRole => handle_query_role(token).await,
        IpcEndpoint::AddRole => handle_add_role(token, payload).await,
        IpcEndpoint::DeleteRole(rid) => handle_delete_role(token, rid).await,
        IpcEndpoint::GetRolesByUser(uid) => handle_get_roles_by_user(token, uid).await,
        IpcEndpoint::GetPermission(pid) => handle_get_permission(token, pid).await,
        IpcEndpoint::QueryPermission => handle_query_permission(token).await,
        IpcEndpoint::AddPermission => handle_add_permission(token, payload).await,
        IpcEndpoint::DeletePermission(pid) => handle_delete_permission(token, pid).await,
        IpcEndpoint::GetPermissionsByRole(rid) => handle_get_permissions_by_role(token, rid).await,
        IpcEndpoint::GetPermissionsByUser(uid) => handle_get_permissions_by_user(token, uid).await,
        IpcEndpoint::AddPermissionForRole(pid, rid) => {
            handle_add_permission_for_role(token, pid, rid).await
        }
        IpcEndpoint::AddRoleForUser(rid, uid) => handle_add_role_for_user(token, rid, uid).await,
        IpcEndpoint::PluginInfo => handle_plugin_info(token).await,
    }
}
