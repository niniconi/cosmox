use reqwest::header::{self, HeaderValue};

use crate::{
    Api, ApiFuture,
    error::SdkError,
    types::{
        InitStatus, InitializeConfig, InstallPlugin, LibrariesRelatedTags, Library, LibraryAdd,
        LibraryDeleteRequest, LibraryModify, LibraryPath, LibraryQueryRequest, LibraryType,
        Message, MessagePayload, Permission, PermissionAddRequest, PluginQueryItem,
        PluginQueryRequest, PushResponse, Resource, ResourceAddRequest, ResourceModifyRequest,
        ResourceQueryRequest, Role, RoleAddRequest, RoleLinkPermissionAddRequest, ScannerInfo,
        ScannerStatus, ScannerTaskAddRequest, SearchRequest, SystemInfo, Tag, TagAddRequest,
        TagCatalogEntry, TagGroup, TagGroupAddRequest, TagGroupDeleteRequest, TagGroupQueryRequest,
        TagQueryRequest, User, UserLogin, UserQueryRequest, UserResp, UserRoleAddRequest,
        UserSignUp,
    },
};

pub struct HttpApi {
    pub base_url: String,
    client: reqwest::Client,
    token: Option<String>,
}

impl HttpApi {
    fn auth_header(&self) -> Option<HeaderValue> {
        self.token
            .as_ref()
            .and_then(|t| HeaderValue::from_str(t).ok())
    }

    async fn get<T: serde::de::DeserializeOwned + std::fmt::Debug>(
        &self,
        path: &str,
    ) -> Result<T, SdkError> {
        let mut req = self.client.get(format!("{}{}", self.base_url, path));
        if let Some(h) = self.auth_header() {
            req = req.header(header::AUTHORIZATION, h);
        }
        let resp = req.send().await.map_err(classify_reqwest_error)?;
        check_status(&resp)?;
        let msg: Message<T> = resp
            .json()
            .await
            .map_err(|e| SdkError::SerdeError(e.to_string()))?;
        Self::extract(msg)
    }

    async fn post<T: serde::de::DeserializeOwned + std::fmt::Debug, B: serde::Serialize>(
        &self,
        path: &str,
        body: &B,
    ) -> Result<T, SdkError> {
        let mut req = self
            .client
            .post(format!("{}{}", self.base_url, path))
            .json(body);
        if let Some(h) = self.auth_header() {
            req = req.header(header::AUTHORIZATION, h);
        }
        let resp = req.send().await.map_err(classify_reqwest_error)?;
        check_status(&resp)?;
        let msg: Message<T> = resp
            .json()
            .await
            .map_err(|e| SdkError::SerdeError(e.to_string()))?;
        Self::extract(msg)
    }

    async fn post_query<T: serde::de::DeserializeOwned + std::fmt::Debug>(
        &self,
        path: &str,
        query_string: &str,
    ) -> Result<T, SdkError> {
        let url = if query_string.is_empty() {
            format!("{}{}", self.base_url, path)
        } else {
            format!("{}{}?{}", self.base_url, path, query_string)
        };
        let mut req = self.client.post(&url);
        if let Some(h) = self.auth_header() {
            req = req.header(header::AUTHORIZATION, h);
        }
        let resp = req.send().await.map_err(classify_reqwest_error)?;
        check_status(&resp)?;
        let msg: Message<T> = resp
            .json()
            .await
            .map_err(|e| SdkError::SerdeError(e.to_string()))?;
        Self::extract(msg)
    }

    async fn post_path<T: serde::de::DeserializeOwned + std::fmt::Debug>(
        &self,
        path: &str,
    ) -> Result<T, SdkError> {
        let mut req = self.client.post(format!("{}{}", self.base_url, path));
        if let Some(h) = self.auth_header() {
            req = req.header(header::AUTHORIZATION, h);
        }
        let resp = req.send().await.map_err(classify_reqwest_error)?;
        check_status(&resp)?;
        let msg: Message<T> = resp
            .json()
            .await
            .map_err(|e| SdkError::SerdeError(e.to_string()))?;
        Self::extract(msg)
    }

    async fn post_binary<T: serde::de::DeserializeOwned + std::fmt::Debug>(
        &self,
        path: &str,
        body: Vec<u8>,
    ) -> Result<T, SdkError> {
        let mut req = self
            .client
            .post(format!("{}{}", self.base_url, path))
            .header(header::CONTENT_TYPE, "application/octet-stream")
            .body(body);
        if let Some(h) = self.auth_header() {
            req = req.header(header::AUTHORIZATION, h);
        }
        let resp = req.send().await.map_err(classify_reqwest_error)?;
        check_status(&resp)?;
        let msg: Message<T> = resp
            .json()
            .await
            .map_err(|e| SdkError::SerdeError(e.to_string()))?;
        Self::extract(msg)
    }

    async fn get_text(&self, path: &str) -> Result<String, SdkError> {
        let mut req = self.client.get(format!("{}{}", self.base_url, path));
        if let Some(h) = self.auth_header() {
            req = req.header(header::AUTHORIZATION, h);
        }
        let resp = req.send().await.map_err(classify_reqwest_error)?;
        check_status(&resp)?;
        resp.text()
            .await
            .map_err(|e| SdkError::Internal(e.to_string()))
    }

    async fn get_vec(&self, path: &str) -> Result<Vec<u8>, SdkError> {
        let mut req = self.client.get(format!("{}{}", self.base_url, path));
        if let Some(h) = self.auth_header() {
            req = req.header(header::AUTHORIZATION, h);
        }
        let resp = req.send().await.map_err(classify_reqwest_error)?;
        check_status(&resp)?;
        Ok(resp
            .bytes()
            .await
            .map_err(|e| SdkError::Internal(e.to_string()))?
            .to_vec())
    }

    fn extract<T: serde::de::DeserializeOwned + std::fmt::Debug>(
        msg: Message<T>,
    ) -> Result<T, SdkError> {
        match msg.payload {
            Some(MessagePayload::Data(data)) => Ok(data),
            Some(MessagePayload::Error(errs)) => {
                Err(SdkError::Internal(format!("API error: {errs:?}")))
            }
            // When no data field is present (e.g. unit `()` returns), try deserializing null.
            // This allows e.g. `extract::<()>(msg)` to succeed when the JSON has no "data" key.
            None => serde_json::from_value(serde_json::Value::Null)
                .map_err(|_| SdkError::Internal("API returned empty payload".into())),
        }
    }
}

fn build_page_query<T: serde::Serialize>(params: &T) -> String {
    let mut pairs: Vec<String> = Vec::new();
    if let Ok(serde_json::Value::Object(map)) = serde_json::to_value(params) {
        for (key, value) in map {
            match value {
                serde_json::Value::String(s) if !s.is_empty() => pairs.push(format!("{key}={s}")),
                serde_json::Value::Number(n) => pairs.push(format!("{key}={n}")),
                serde_json::Value::Bool(b) => pairs.push(format!("{key}={b}")),
                _ => {}
            }
        }
    }
    if pairs.is_empty() {
        String::new()
    } else {
        format!("?{}", pairs.join("&"))
    }
}

fn classify_reqwest_error(e: reqwest::Error) -> SdkError {
    if e.is_connect() || e.is_timeout() {
        SdkError::ConnectionFailed(e.to_string())
    } else if let Some(status) = e.status() {
        if status.as_u16() == 401 {
            SdkError::Unauthenticated
        } else {
            SdkError::HttpError {
                status: status.as_u16() as i32,
                message: e.to_string(),
            }
        }
    } else {
        SdkError::Internal(e.to_string())
    }
}

fn check_status(resp: &reqwest::Response) -> Result<(), SdkError> {
    let status = resp.status().as_u16();
    if status == 401 {
        Err(SdkError::Unauthenticated)
    } else if status >= 400 {
        Err(SdkError::HttpError {
            status: status as i32,
            message: format!("HTTP {}", status),
        })
    } else {
        Ok(())
    }
}

impl Api for HttpApi {
    fn new(hostname: &'static str, port: u16) -> Self {
        Self {
            base_url: format!("http://{hostname}:{port}/api"),
            client: reqwest::Client::builder()
                .cookie_store(true)
                .build()
                .expect("reqwest Client::builder()"),
            token: None,
        }
    }

    fn set_token(&mut self, token: String) {
        self.token = Some(token);
    }

    fn get_token(&self) -> Option<String> {
        self.token.clone()
    }

    fn logout(&mut self) {
        self.token = None;
    }

    fn login(&mut self, payload: UserLogin) -> ApiFuture<'_, ()> {
        Box::pin(async move {
            let token: String = self.post("/user/login", &payload).await?;
            self.token = Some(token);
            Ok(())
        })
    }

    fn system_info(&self) -> ApiFuture<'_, SystemInfo> {
        Box::pin(async move { self.get("/system/info").await })
    }

    fn system_about(&self) -> ApiFuture<'_, String> {
        Box::pin(async move { self.get("/system/about").await })
    }

    fn system_log(&self) -> ApiFuture<'_, String> {
        Box::pin(async move { self.get("/system/log").await })
    }

    fn system_restart(&self) -> ApiFuture<'_, ()> {
        Box::pin(async move {
            let _: String = self.post_path("/system/restart").await?;
            Ok(())
        })
    }

    fn system_shutdown(&self) -> ApiFuture<'_, ()> {
        Box::pin(async move {
            let _: String = self.post_path("/system/shutdown").await?;
            Ok(())
        })
    }

    fn system_delete_all(&self) -> ApiFuture<'_, ()> {
        Box::pin(async move { self.post_path("/system/all/delete").await })
    }

    fn user_get(&self, uid: u64) -> ApiFuture<'_, User> {
        Box::pin(async move { self.get(&format!("/user/{uid}")).await })
    }

    fn user_query(&self, params: UserQueryRequest) -> ApiFuture<'_, Vec<User>> {
        Box::pin(async move {
            let qs = build_page_query(&params);
            self.get(&format!("/user/query{qs}")).await
        })
    }

    fn user_register(&self, payload: UserSignUp) -> ApiFuture<'_, UserResp> {
        Box::pin(async move { self.post("/user/register", &payload).await })
    }

    fn user_delete(&self, uid: u64) -> ApiFuture<'_, ()> {
        Box::pin(async move { self.post_query("/user/delete", &format!("uid={uid}")).await })
    }

    fn user_role_add(&self, payload: UserRoleAddRequest) -> ApiFuture<'_, ()> {
        Box::pin(async move { self.post("/user/role/add", &payload).await })
    }

    fn library_get(&self, lid: u64) -> ApiFuture<'_, Library> {
        Box::pin(async move { self.get(&format!("/library/{lid}")).await })
    }

    fn library_query(&self, params: LibraryQueryRequest) -> ApiFuture<'_, Vec<Library>> {
        Box::pin(async move {
            let qs = build_page_query(&params);
            self.get(&format!("/library/query{qs}")).await
        })
    }

    fn library_add(
        &self,
        payload: LibraryAdd,
    ) -> ApiFuture<'_, (Library, Vec<LibrariesRelatedTags>, Vec<LibraryPath>)> {
        Box::pin(async move { self.post("/library/add", &payload).await })
    }

    fn library_modify(&self, lid: u64, payload: LibraryModify) -> ApiFuture<'_, ()> {
        Box::pin(async move { self.post(&format!("/library/{lid}/modify"), &payload).await })
    }

    fn library_delete(&self, payload: LibraryDeleteRequest) -> ApiFuture<'_, ()> {
        Box::pin(async move {
            self.post_query("/library/delete", &format!("lid={}", payload.lid))
                .await
        })
    }

    fn library_type_all(&self) -> ApiFuture<'_, Vec<LibraryType>> {
        Box::pin(async move { self.get("/library/types/all").await })
    }

    fn tag_get(&self, tid: u64) -> ApiFuture<'_, Tag> {
        Box::pin(async move { self.get(&format!("/tag/{tid}")).await })
    }

    fn tag_add(&self, payload: TagAddRequest) -> ApiFuture<'_, u64> {
        Box::pin(async move { self.post("/tag/add", &payload).await })
    }

    fn tag_query(&self, params: TagQueryRequest) -> ApiFuture<'_, Vec<Tag>> {
        Box::pin(async move {
            let qs = build_page_query(&params);
            self.get(&format!("/tag/query{qs}")).await
        })
    }

    fn tag_group_get(&self, tgid: u64) -> ApiFuture<'_, TagGroup> {
        Box::pin(async move { self.get(&format!("/tag/group/{tgid}")).await })
    }

    fn tag_group_add(&self, payload: TagGroupAddRequest) -> ApiFuture<'_, u64> {
        Box::pin(async move { self.post("/tag/group/add", &payload).await })
    }

    fn tag_group_delete(&self, payload: TagGroupDeleteRequest) -> ApiFuture<'_, ()> {
        Box::pin(async move {
            self.post_query("/tag/group/delete", &format!("tgid={}", payload.tgid))
                .await
        })
    }

    fn tag_group_query(&self, params: TagGroupQueryRequest) -> ApiFuture<'_, Vec<TagGroup>> {
        Box::pin(async move {
            let qs = build_page_query(&params);
            self.get(&format!("/tag/group/query{qs}")).await
        })
    }

    fn tag_catalog(&self) -> ApiFuture<'_, Vec<TagCatalogEntry>> {
        Box::pin(async move { self.get("/tag/catalog").await })
    }

    fn resource_get(&self, rid: u64) -> ApiFuture<'_, Resource> {
        Box::pin(async move { self.get(&format!("/resource/{rid}")).await })
    }

    fn resource_query(&self, params: ResourceQueryRequest) -> ApiFuture<'_, Vec<Resource>> {
        Box::pin(async move {
            let qs = build_page_query(&params);
            self.get(&format!("/resource/query{qs}")).await
        })
    }

    fn resource_add(&self, payload: ResourceAddRequest) -> ApiFuture<'_, u64> {
        Box::pin(async move { self.post("/resource/add", &payload).await })
    }

    fn resource_modify(&self, rid: u64, payload: ResourceModifyRequest) -> ApiFuture<'_, ()> {
        Box::pin(async move {
            let _: String = self
                .post(&format!("/resource/{rid}/modify"), &payload)
                .await?;
            Ok(())
        })
    }

    fn resource_delete(&self, rid: u64) -> ApiFuture<'_, ()> {
        Box::pin(async move {
            self.post_query("/resource/delete", &format!("rid={rid}"))
                .await
        })
    }

    fn resource_add_tag(&self, rid: u64, tag_ids: Vec<u64>) -> ApiFuture<'_, serde_json::Value> {
        Box::pin(async move {
            self.post(
                &format!("/resource/{rid}/tag/add"),
                &serde_json::json!({ "tags": tag_ids }),
            )
            .await
        })
    }

    fn resource_get_metadata(&self, rid: u64) -> ApiFuture<'_, serde_json::Value> {
        Box::pin(async move { self.get(&format!("/resource/{rid}/metadata")).await })
    }

    fn acl_query_role(&self) -> ApiFuture<'_, Vec<Role>> {
        Box::pin(async move { self.get("/user/acl/query/role").await })
    }

    fn acl_query_permission(&self) -> ApiFuture<'_, Vec<Permission>> {
        Box::pin(async move { self.get("/user/acl/query/permission").await })
    }

    fn acl_add_role(&self, payload: RoleAddRequest) -> ApiFuture<'_, ()> {
        Box::pin(async move { self.post("/user/acl/role/add", &payload).await })
    }

    fn acl_delete_role(&self, rid: u64) -> ApiFuture<'_, ()> {
        Box::pin(async move {
            self.post_query("/user/acl/role/delete", &format!("rid={rid}"))
                .await
        })
    }

    fn acl_add_permission(&self, payload: PermissionAddRequest) -> ApiFuture<'_, ()> {
        Box::pin(async move { self.post("/user/acl/permission/add", &payload).await })
    }

    fn acl_delete_permission(&self, pid: u64) -> ApiFuture<'_, ()> {
        Box::pin(async move {
            self.post_query("/user/acl/permission/delete", &format!("pid={pid}"))
                .await
        })
    }

    fn acl_add_permission_for_role(
        &self,
        payload: RoleLinkPermissionAddRequest,
    ) -> ApiFuture<'_, ()> {
        Box::pin(async move { self.post("/user/acl/role/permission/add", &payload).await })
    }

    fn plugin_info(&self) -> ApiFuture<'_, String> {
        Box::pin(async move { self.get("/plugin/info").await })
    }

    fn plugin_query(&self, params: PluginQueryRequest) -> ApiFuture<'_, Vec<PluginQueryItem>> {
        Box::pin(async move {
            let qs = build_page_query(&params);
            self.get(&format!("/plugin/query{qs}")).await
        })
    }

    fn plugin_install(&self, payload: InstallPlugin) -> ApiFuture<'_, String> {
        Box::pin(async move {
            match payload {
                InstallPlugin::Url(url) => {
                    self.post_query("/plugin/install", &format!("url={url}"))
                        .await
                }
                InstallPlugin::Data(data) => self.post_binary("/plugin/install", data).await,
            }
        })
    }

    fn plugin_uninstall(&self, _name: String) -> ApiFuture<'_, ()> {
        Box::pin(async move { self.post_path("/plugin/uninstall").await })
    }

    fn plugin_enable(&self, name: String) -> ApiFuture<'_, ()> {
        Box::pin(async move { self.post_path(&format!("/plugin/{name}/enable")).await })
    }

    fn plugin_disable(&self, name: String) -> ApiFuture<'_, ()> {
        Box::pin(async move { self.post_path(&format!("/plugin/{name}/disable")).await })
    }

    fn scanner_scan(&self, lid: u64) -> ApiFuture<'_, String> {
        Box::pin(async move { self.post_path(&format!("/scanner/scan/{lid}")).await })
    }

    fn scanner_scan_all(&self) -> ApiFuture<'_, String> {
        Box::pin(async move { self.post_path("/scanner/scan/all").await })
    }

    fn scanner_get_status(&self) -> ApiFuture<'_, ScannerStatus> {
        Box::pin(async move { self.get("/scanner/status").await })
    }

    fn scanner_info(&self) -> ApiFuture<'_, ScannerInfo> {
        Box::pin(async move { self.get("/scanner/info").await })
    }

    fn scanner_add_task(&self, payload: ScannerTaskAddRequest) -> ApiFuture<'_, ()> {
        Box::pin(async move { self.post("/scanner/task/add", &payload).await })
    }

    fn metadata_query(&self, root_node: u64, depth: usize) -> ApiFuture<'_, serde_json::Value> {
        Box::pin(async move {
            self.get(&format!(
                "/metadata/query?root_node={root_node}&depth={depth}"
            ))
            .await
        })
    }

    fn metadata_get(&self, rid: u64) -> ApiFuture<'_, serde_json::Value> {
        Box::pin(async move { self.get(&format!("/metadata/{rid}")).await })
    }

    fn path_sub_path(&self, path: String, show_hide: bool) -> ApiFuture<'_, Vec<String>> {
        Box::pin(async move {
            self.get(&format!(
                "/scanner/path/list?path={path}&show_hide={show_hide}"
            ))
            .await
        })
    }

    fn initialize(&self, payload: InitializeConfig) -> ApiFuture<'_, InitStatus> {
        Box::pin(async move { self.post("/initialize", &payload).await })
    }

    fn search(&self, params: SearchRequest) -> ApiFuture<'_, serde_json::Value> {
        Box::pin(async move {
            let mut parts: Vec<String> = Vec::new();
            parts.push(format!("keyword={}", params.keyword));
            if let Some(t) = &params.tags {
                parts.push(format!("tags={}", t.join(",")));
            }
            if let Some(l) = params.lid {
                parts.push(format!("lid={l}"));
            }
            if let Some(d) = &params.before_create_datetime {
                parts.push(format!("before_create_datetime={d}"));
            }
            if let Some(d) = &params.after_create_datetime {
                parts.push(format!("after_create_datetime={d}"));
            }
            if let Some(d) = &params.before_last_update_datetime {
                parts.push(format!("before_last_update_datetime={d}"));
            }
            if let Some(d) = &params.after_last_update_datetime {
                parts.push(format!("after_last_update_datetime={d}"));
            }
            if let Some(s) = &params.sort {
                parts.push(format!("sort_by={s}"));
            }
            if let Some(p) = params.page {
                parts.push(format!("page={p}"));
            }
            parts.push(format!("page_size={}", params.page_size));
            let qs = parts.join("&");
            self.get(&format!("/search?{qs}")).await
        })
    }

    fn openapi(&self) -> ApiFuture<'_, String> {
        Box::pin(async move { self.get_text("/openapi.yaml").await })
    }

    fn docs(&self) -> ApiFuture<'_, String> {
        Box::pin(async move { self.get_text("/docs").await })
    }

    fn user_upload_avatar(&self, uid: u64, data: Vec<u8>) -> ApiFuture<'_, PushResponse> {
        Box::pin(async move {
            self.post_binary(&format!("/user/{uid}/avatar/upload"), data)
                .await
        })
    }

    fn item_push(&self, data: Vec<u8>) -> ApiFuture<'_, PushResponse> {
        Box::pin(async move { self.post_binary("/item/push", data).await })
    }

    fn item_pull(&self, id: u64) -> ApiFuture<'_, Vec<u8>> {
        Box::pin(async move { self.get_vec(&format!("/item/{id}/pull")).await })
    }
}
