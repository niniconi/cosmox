use chrono::{DateTime, NaiveDateTime, Utc};
use serde::{Deserialize, Serialize};

// -- API Envelope --

#[derive(Debug, Deserialize)]
pub struct Message<T> {
    pub code: String,
    pub message: String,
    pub status: String,
    pub datetime: DateTime<Utc>,
    #[serde(flatten)]
    pub payload: Option<MessagePayload<T>>,
    pub pagination: Option<Pagination>,
}

#[derive(Debug, Deserialize)]
pub enum MessagePayload<T> {
    #[serde(rename = "errors")]
    Error(Vec<T>),
    #[serde(rename = "data")]
    Data(T),
}

#[derive(Debug, Deserialize)]
pub struct Pagination {
    pub total_items: u64,
    pub total_pages: u64,
    pub current_page: u64,
    pub page_size: u64,
    pub next_page_url: String,
    pub prev_page_url: String,
}

// -- Auth --

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum UserLoginIdent {
    Username(String),
    Email(String),
}

#[derive(Debug, Serialize)]
pub struct UserLogin {
    #[serde(flatten)]
    pub ident: UserLoginIdent,
    pub password: String,
}

// -- System --

#[derive(Debug, Deserialize)]
pub struct SystemInfo {
    pub os: String,
    pub name: String,
    pub version: String,
    pub author: String,
    pub is_first_boot: bool,
}

// -- User --

#[derive(Debug, Deserialize)]
pub struct User {
    pub uid: u64,
    pub username: String,
    pub email: Option<String>,
    pub nickname: Option<String>,
    pub avatar: Option<u64>,
    pub create_datetime: NaiveDateTime,
    pub last_update_datetime: NaiveDateTime,
}

#[derive(Debug, Serialize)]
pub struct UserSignUp {
    pub username: String,
    pub password: String,
    pub confirm_password: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub nickname: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UserResp {
    pub uid: u64,
    pub username: String,
    pub email: Option<String>,
}

// -- Library --

#[derive(Debug, Deserialize)]
pub struct LibraryType {
    pub tid: u64,
    pub scan_mode: Option<String>,
    pub label: String,
}

#[derive(Debug, Deserialize)]
pub struct Library {
    pub lid: u64,
    pub name: Option<String>,
    pub description: Option<String>,
    pub r#type: Option<u64>,
    pub create_by_uid: u64,
    pub create_datetime: NaiveDateTime,
    pub last_update_datetime: NaiveDateTime,
}

#[derive(Debug, Deserialize)]
pub struct LibrariesRelatedTags {
    pub lrtid: u64,
    pub lid: u64,
    pub tid: u64,
}

#[derive(Debug, Deserialize)]
pub struct LibraryPath {
    pub lpid: u64,
    pub lid: u64,
    pub path: String,
}

#[derive(Debug, Serialize)]
pub struct LibraryAdd {
    pub name: String,
    pub description: Option<String>,
    pub r#type: u64,
    pub tags: Vec<u64>,
    pub library_paths: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct LibraryModify {
    pub name: Option<String>,
    pub description: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LibraryDeleteRequest {
    pub lid: u64,
}

// -- Tag --

#[derive(Debug, Deserialize)]
pub struct Tag {
    pub tid: u64,
    #[serde(rename = "text")]
    pub name: String,
    pub tgid: u64,
    pub create_datetime: NaiveDateTime,
}

#[derive(Debug, Serialize)]
pub struct TagAddRequest {
    pub label: String,
    pub tgid: u64,
}

// -- Tag Group --

#[derive(Debug, Deserialize)]
pub struct TagGroup {
    pub tgid: u64,
    #[serde(rename = "text")]
    pub name: String,
    pub create_datetime: NaiveDateTime,
}

#[derive(Debug, Serialize)]
pub struct TagGroupAddRequest {
    pub label: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TagGroupDeleteRequest {
    pub tgid: u64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TagGroupQueryRequest {
    pub tgid: Option<u64>,
    #[serde(rename = "sort_by")]
    pub sort: Option<String>,
    pub page: Option<u64>,
    #[serde(default = "default_page_size")]
    pub page_size: u64,
}

#[derive(Debug, Deserialize)]
pub struct TagCatalogEntry {
    pub group: TagGroup,
    pub tags: Vec<Tag>,
}

// -- Resource --

#[derive(Debug, Deserialize)]
pub struct Resource {
    pub rid: u64,
    pub name: Option<String>,
    pub description: Option<String>,
    pub lid: Option<u64>,
    pub create_datetime: NaiveDateTime,
    pub last_update_datetime: NaiveDateTime,
    pub metadata_index: Option<u64>,
    pub cover: Option<u64>,
}

#[derive(Debug, Serialize)]
pub struct ResourceAddRequest {
    pub name: String,
    pub lid: u64,
    pub description: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ResourceModifyRequest {
    pub name: Option<String>,
    pub description: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ResourceDeleteRequest {
    pub rid: u64,
}

// -- Acl --

#[derive(Debug, Deserialize)]
pub struct Role {
    pub rid: u64,
    pub name: String,
    pub description: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct RoleAddRequest {
    pub name: String,
    pub description: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct Permission {
    pub pid: u64,
    pub name: String,
    pub description: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct PermissionAddRequest {
    pub name: String,
    pub description: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct RoleLinkPermissionAddRequest {
    pub rid: u64,
    pub pid: u64,
}

// -- User ACL --

#[derive(Debug, Serialize)]
pub struct UserRoleAddRequest {
    pub uid: u64,
    pub rid: u64,
}

// -- Init --

#[derive(Debug, Serialize)]
pub struct InitializeConfig {
    pub admin_password: String,
    pub admin_confirm_password: String,
}

#[derive(Debug, Deserialize)]
pub struct InitStatus {
    pub initialized: bool,
}

// -- Plugin --

#[derive(Debug, Serialize)]
pub struct InstallPlugin {
    pub url: Option<String>,
}

// -- Scanner --

#[derive(Debug, Serialize)]
pub struct ScannerTaskAddRequest {
    pub lid: Option<u64>,
    pub full_scan: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct ScannerStatus {
    pub scanning: bool,
    pub current_lid: Option<u64>,
}

#[derive(Debug, Deserialize)]
pub struct ScannerInfo {
    pub available: bool,
    pub version: Option<String>,
}

// -- Metadata --

#[derive(Debug, Deserialize)]
pub struct MetadataQueryRequest {
    pub root_node: u64,
    pub depth: usize,
}

// -- File --

#[derive(Debug, Deserialize)]
pub struct PushResponse {
    pub pmid: u64,
    pub uploaded_size: u64,
}

// -- Query params (page_helper) --

#[derive(Debug, Serialize, Deserialize)]
pub struct UserQueryRequest {
    pub status: Option<String>,
    pub role: Option<String>,
    pub search: Option<String>,
    #[serde(rename = "sort_by")]
    pub sort: Option<String>,
    pub page: Option<u64>,
    #[serde(default = "default_page_size")]
    pub page_size: u64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TagQueryRequest {
    pub tid: Option<u64>,
    #[serde(rename = "sort_by")]
    pub sort: Option<String>,
    pub page: Option<u64>,
    #[serde(default = "default_page_size")]
    pub page_size: u64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ResourceQueryRequest {
    pub lid: u64,
    #[serde(rename = "sort_by")]
    pub sort: Option<String>,
    pub page: Option<u64>,
    #[serde(default = "default_page_size")]
    pub page_size: u64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LibraryQueryRequest {
    #[serde(rename = "sort_by")]
    pub sort: Option<String>,
    pub page: Option<u64>,
    #[serde(default = "default_page_size")]
    pub page_size: u64,
}

fn default_page_size() -> u64 {
    40
}

// -- Search --

#[derive(Debug, Serialize)]
pub struct SearchRequest {
    pub keyword: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lid: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub before_create_datetime: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub after_create_datetime: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub before_last_update_datetime: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub after_last_update_datetime: Option<String>,
    #[serde(rename = "sort_by", skip_serializing_if = "Option::is_none")]
    pub sort: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub page: Option<u64>,
    #[serde(default = "default_page_size")]
    pub page_size: u64,
}
