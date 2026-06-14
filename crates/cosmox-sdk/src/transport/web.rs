use std::pin::Pin;

use reqwest::header::{self, HeaderValue};

use crate::{
    Api,
    error::SdkError,
    types::{
        InitStatus, InitializeConfig, InstallPlugin, LibrariesRelatedTags, Library, LibraryAdd,
        LibraryDeleteRequest, LibraryModify, LibraryPath, LibraryQueryRequest, LibraryType,
        Message, MessagePayload, Permission, PermissionAddRequest, PushResponse, Resource,
        ResourceAddRequest, ResourceModifyRequest, ResourceQueryRequest, Role, RoleAddRequest,
        RoleLinkPermissionAddRequest, ScannerInfo, ScannerStatus, ScannerTaskAddRequest,
        SearchRequest, SystemInfo, Tag, TagAddRequest, TagCatalogEntry, TagGroup,
        TagGroupAddRequest, TagGroupDeleteRequest, TagGroupQueryRequest, TagQueryRequest, User,
        UserLogin, UserQueryRequest, UserResp, UserRoleAddRequest, UserSignUp,
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

    fn login(
        &mut self,
        payload: UserLogin,
    ) -> Pin<Box<dyn Future<Output = Result<(), SdkError>> + Send + '_>> {
        Box::pin(async move {
            let token: String = self.post("/user/login", &payload).await?;
            self.token = Some(token);
            Ok(())
        })
    }

    fn system_info(
        &self,
    ) -> Pin<Box<dyn Future<Output = Result<SystemInfo, SdkError>> + Send + '_>> {
        Box::pin(async move { self.get("/system/info").await })
    }

    fn system_about(&self) -> Pin<Box<dyn Future<Output = Result<String, SdkError>> + Send + '_>> {
        Box::pin(async move { self.get("/system/about").await })
    }

    fn system_log(&self) -> Pin<Box<dyn Future<Output = Result<String, SdkError>> + Send + '_>> {
        Box::pin(async move { self.get("/system/log").await })
    }

    fn system_restart(&self) -> Pin<Box<dyn Future<Output = Result<(), SdkError>> + Send + '_>> {
        Box::pin(async move {
            let _: String = self.post_path("/system/restart").await?;
            Ok(())
        })
    }

    fn system_shutdown(&self) -> Pin<Box<dyn Future<Output = Result<(), SdkError>> + Send + '_>> {
        Box::pin(async move {
            let _: String = self.post_path("/system/shutdown").await?;
            Ok(())
        })
    }

    fn system_delete_all(&self) -> Pin<Box<dyn Future<Output = Result<(), SdkError>> + Send + '_>> {
        Box::pin(async move { self.post_path("/system/all/delete").await })
    }

    fn user_get(
        &self,
        uid: u64,
    ) -> Pin<Box<dyn Future<Output = Result<User, SdkError>> + Send + '_>> {
        Box::pin(async move { self.get(&format!("/user/{uid}")).await })
    }

    fn user_query(
        &self,
        params: UserQueryRequest,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<User>, SdkError>> + Send + '_>> {
        Box::pin(async move {
            let qs = build_page_query(&params);
            self.get(&format!("/user/query{qs}")).await
        })
    }

    fn user_register(
        &self,
        payload: UserSignUp,
    ) -> Pin<Box<dyn Future<Output = Result<UserResp, SdkError>> + Send + '_>> {
        Box::pin(async move { self.post("/user/register", &payload).await })
    }

    fn user_delete(
        &self,
        uid: u64,
    ) -> Pin<Box<dyn Future<Output = Result<(), SdkError>> + Send + '_>> {
        Box::pin(async move { self.post_query("/user/delete", &format!("uid={uid}")).await })
    }

    fn user_role_add(
        &self,
        payload: UserRoleAddRequest,
    ) -> Pin<Box<dyn Future<Output = Result<(), SdkError>> + Send + '_>> {
        Box::pin(async move { self.post("/user/role/add", &payload).await })
    }

    fn library_get(
        &self,
        lid: u64,
    ) -> Pin<Box<dyn Future<Output = Result<Library, SdkError>> + Send + '_>> {
        Box::pin(async move { self.get(&format!("/library/{lid}")).await })
    }

    fn library_query(
        &self,
        params: LibraryQueryRequest,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<Library>, SdkError>> + Send + '_>> {
        Box::pin(async move {
            let qs = build_page_query(&params);
            self.get(&format!("/library/query{qs}")).await
        })
    }

    fn library_add(
        &self,
        payload: LibraryAdd,
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
        Box::pin(async move { self.post("/library/add", &payload).await })
    }

    fn library_modify(
        &self,
        lid: u64,
        payload: LibraryModify,
    ) -> Pin<Box<dyn Future<Output = Result<(), SdkError>> + Send + '_>> {
        Box::pin(async move { self.post(&format!("/library/{lid}/modify"), &payload).await })
    }

    fn library_delete(
        &self,
        payload: LibraryDeleteRequest,
    ) -> Pin<Box<dyn Future<Output = Result<(), SdkError>> + Send + '_>> {
        Box::pin(async move {
            self.post_query("/library/delete", &format!("lid={}", payload.lid))
                .await
        })
    }

    fn library_type_all(
        &self,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<LibraryType>, SdkError>> + Send + '_>> {
        Box::pin(async move { self.get("/library/types/all").await })
    }

    fn tag_get(
        &self,
        tid: u64,
    ) -> Pin<Box<dyn Future<Output = Result<Tag, SdkError>> + Send + '_>> {
        Box::pin(async move { self.get(&format!("/tag/{tid}")).await })
    }

    fn tag_add(
        &self,
        payload: TagAddRequest,
    ) -> Pin<Box<dyn Future<Output = Result<u64, SdkError>> + Send + '_>> {
        Box::pin(async move { self.post("/tag/add", &payload).await })
    }

    fn tag_query(
        &self,
        params: TagQueryRequest,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<Tag>, SdkError>> + Send + '_>> {
        Box::pin(async move {
            let qs = build_page_query(&params);
            self.get(&format!("/tag/query{qs}")).await
        })
    }

    fn tag_group_get(
        &self,
        tgid: u64,
    ) -> Pin<Box<dyn Future<Output = Result<TagGroup, SdkError>> + Send + '_>> {
        Box::pin(async move { self.get(&format!("/tag/group/{tgid}")).await })
    }

    fn tag_group_add(
        &self,
        payload: TagGroupAddRequest,
    ) -> Pin<Box<dyn Future<Output = Result<u64, SdkError>> + Send + '_>> {
        Box::pin(async move { self.post("/tag/group/add", &payload).await })
    }

    fn tag_group_delete(
        &self,
        payload: TagGroupDeleteRequest,
    ) -> Pin<Box<dyn Future<Output = Result<(), SdkError>> + Send + '_>> {
        Box::pin(async move {
            self.post_query("/tag/group/delete", &format!("tgid={}", payload.tgid))
                .await
        })
    }

    fn tag_group_query(
        &self,
        params: TagGroupQueryRequest,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<TagGroup>, SdkError>> + Send + '_>> {
        Box::pin(async move {
            let qs = build_page_query(&params);
            self.get(&format!("/tag/group/query{qs}")).await
        })
    }

    fn tag_catalog(
        &self,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<TagCatalogEntry>, SdkError>> + Send + '_>> {
        Box::pin(async move { self.get("/tag/catalog").await })
    }

    fn resource_get(
        &self,
        rid: u64,
    ) -> Pin<Box<dyn Future<Output = Result<Resource, SdkError>> + Send + '_>> {
        Box::pin(async move { self.get(&format!("/resource/{rid}")).await })
    }

    fn resource_query(
        &self,
        params: ResourceQueryRequest,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<Resource>, SdkError>> + Send + '_>> {
        Box::pin(async move {
            let qs = build_page_query(&params);
            self.get(&format!("/resource/query{qs}")).await
        })
    }

    fn resource_add(
        &self,
        payload: ResourceAddRequest,
    ) -> Pin<Box<dyn Future<Output = Result<u64, SdkError>> + Send + '_>> {
        Box::pin(async move { self.post("/resource/add", &payload).await })
    }

    fn resource_modify(
        &self,
        rid: u64,
        payload: ResourceModifyRequest,
    ) -> Pin<Box<dyn Future<Output = Result<(), SdkError>> + Send + '_>> {
        Box::pin(async move {
            let _: String = self
                .post(&format!("/resource/{rid}/modify"), &payload)
                .await?;
            Ok(())
        })
    }

    fn resource_delete(
        &self,
        rid: u64,
    ) -> Pin<Box<dyn Future<Output = Result<(), SdkError>> + Send + '_>> {
        Box::pin(async move {
            self.post_query("/resource/delete", &format!("rid={rid}"))
                .await
        })
    }

    fn resource_add_tag(
        &self,
        rid: u64,
        tag_ids: Vec<u64>,
    ) -> Pin<Box<dyn Future<Output = Result<serde_json::Value, SdkError>> + Send + '_>> {
        Box::pin(async move {
            self.post(
                &format!("/resource/{rid}/tag/add"),
                &serde_json::json!({ "tags": tag_ids }),
            )
            .await
        })
    }

    fn resource_get_metadata(
        &self,
        rid: u64,
    ) -> Pin<Box<dyn Future<Output = Result<serde_json::Value, SdkError>> + Send + '_>> {
        Box::pin(async move { self.get(&format!("/resource/{rid}/metadata")).await })
    }

    fn acl_query_role(
        &self,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<Role>, SdkError>> + Send + '_>> {
        Box::pin(async move { self.get("/user/acl/query/role").await })
    }

    fn acl_query_permission(
        &self,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<Permission>, SdkError>> + Send + '_>> {
        Box::pin(async move { self.get("/user/acl/query/permission").await })
    }

    fn acl_add_role(
        &self,
        payload: RoleAddRequest,
    ) -> Pin<Box<dyn Future<Output = Result<(), SdkError>> + Send + '_>> {
        Box::pin(async move { self.post("/user/acl/role/add", &payload).await })
    }

    fn acl_delete_role(
        &self,
        rid: u64,
    ) -> Pin<Box<dyn Future<Output = Result<(), SdkError>> + Send + '_>> {
        Box::pin(async move {
            self.post_query("/user/acl/role/delete", &format!("rid={rid}"))
                .await
        })
    }

    fn acl_add_permission(
        &self,
        payload: PermissionAddRequest,
    ) -> Pin<Box<dyn Future<Output = Result<(), SdkError>> + Send + '_>> {
        Box::pin(async move { self.post("/user/acl/permission/add", &payload).await })
    }

    fn acl_delete_permission(
        &self,
        pid: u64,
    ) -> Pin<Box<dyn Future<Output = Result<(), SdkError>> + Send + '_>> {
        Box::pin(async move {
            self.post_query("/user/acl/permission/delete", &format!("pid={pid}"))
                .await
        })
    }

    fn acl_add_permission_for_role(
        &self,
        payload: RoleLinkPermissionAddRequest,
    ) -> Pin<Box<dyn Future<Output = Result<(), SdkError>> + Send + '_>> {
        Box::pin(async move { self.post("/user/acl/role/permission/add", &payload).await })
    }

    fn plugin_info(&self) -> Pin<Box<dyn Future<Output = Result<String, SdkError>> + Send + '_>> {
        Box::pin(async move { self.get("/plugin/info").await })
    }

    fn plugin_install(
        &self,
        payload: InstallPlugin,
    ) -> Pin<Box<dyn Future<Output = Result<String, SdkError>> + Send + '_>> {
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

    fn plugin_uninstall(
        &self,
        _id: u64,
    ) -> Pin<Box<dyn Future<Output = Result<(), SdkError>> + Send + '_>> {
        Box::pin(async move { self.post_path("/plugin/uninstall").await })
    }

    fn plugin_enable(
        &self,
        id: u64,
    ) -> Pin<Box<dyn Future<Output = Result<(), SdkError>> + Send + '_>> {
        Box::pin(async move { self.post_path(&format!("/plugin/{id}/enable")).await })
    }

    fn plugin_disable(
        &self,
        id: u64,
    ) -> Pin<Box<dyn Future<Output = Result<(), SdkError>> + Send + '_>> {
        Box::pin(async move { self.post_path(&format!("/plugin/{id}/disable")).await })
    }

    fn scanner_scan(
        &self,
        lid: u64,
    ) -> Pin<Box<dyn Future<Output = Result<String, SdkError>> + Send + '_>> {
        Box::pin(async move { self.post_path(&format!("/scanner/scan/{lid}")).await })
    }

    fn scanner_scan_all(
        &self,
    ) -> Pin<Box<dyn Future<Output = Result<String, SdkError>> + Send + '_>> {
        Box::pin(async move { self.post_path("/scanner/scan/all").await })
    }

    fn scanner_get_status(
        &self,
    ) -> Pin<Box<dyn Future<Output = Result<ScannerStatus, SdkError>> + Send + '_>> {
        Box::pin(async move { self.get("/scanner/status").await })
    }

    fn scanner_info(
        &self,
    ) -> Pin<Box<dyn Future<Output = Result<ScannerInfo, SdkError>> + Send + '_>> {
        Box::pin(async move { self.get("/scanner/info").await })
    }

    fn scanner_add_task(
        &self,
        payload: ScannerTaskAddRequest,
    ) -> Pin<Box<dyn Future<Output = Result<(), SdkError>> + Send + '_>> {
        Box::pin(async move { self.post("/scanner/task/add", &payload).await })
    }

    fn metadata_query(
        &self,
        root_node: u64,
        depth: usize,
    ) -> Pin<Box<dyn Future<Output = Result<serde_json::Value, SdkError>> + Send + '_>> {
        Box::pin(async move {
            self.get(&format!(
                "/metadata/query?root_node={root_node}&depth={depth}"
            ))
            .await
        })
    }

    fn metadata_get(
        &self,
        rid: u64,
    ) -> Pin<Box<dyn Future<Output = Result<serde_json::Value, SdkError>> + Send + '_>> {
        Box::pin(async move { self.get(&format!("/metadata/{rid}")).await })
    }

    fn path_sub_path(
        &self,
        path: String,
        show_hide: bool,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<String>, SdkError>> + Send + '_>> {
        Box::pin(async move {
            self.get(&format!(
                "/scanner/path/list?path={path}&show_hide={show_hide}"
            ))
            .await
        })
    }

    fn initialize(
        &self,
        payload: InitializeConfig,
    ) -> Pin<Box<dyn Future<Output = Result<InitStatus, SdkError>> + Send + '_>> {
        Box::pin(async move { self.post("/initialize", &payload).await })
    }

    fn search(
        &self,
        params: SearchRequest,
    ) -> Pin<Box<dyn Future<Output = Result<serde_json::Value, SdkError>> + Send + '_>> {
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

    fn openapi(&self) -> Pin<Box<dyn Future<Output = Result<String, SdkError>> + Send + '_>> {
        Box::pin(async move { self.get_text("/openapi.yaml").await })
    }

    fn docs(&self) -> Pin<Box<dyn Future<Output = Result<String, SdkError>> + Send + '_>> {
        Box::pin(async move { self.get_text("/docs").await })
    }

    fn user_upload_avatar(
        &self,
        uid: u64,
        data: Vec<u8>,
    ) -> Pin<Box<dyn Future<Output = Result<PushResponse, SdkError>> + Send + '_>> {
        Box::pin(async move {
            self.post_binary(&format!("/user/{uid}/avatar/upload"), data)
                .await
        })
    }

    fn item_push(
        &self,
        data: Vec<u8>,
    ) -> Pin<Box<dyn Future<Output = Result<PushResponse, SdkError>> + Send + '_>> {
        Box::pin(async move { self.post_binary("/item/push", data).await })
    }

    fn item_pull(
        &self,
        id: u64,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<u8>, SdkError>> + Send + '_>> {
        Box::pin(async move { self.get_vec(&format!("/item/{id}/pull")).await })
    }
}
