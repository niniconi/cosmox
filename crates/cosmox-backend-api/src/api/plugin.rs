use bytes::Bytes;
use cosmox_macros::page_helper;
use futures_util::StreamExt;
use serde::{Deserialize, Serialize};

use cosmox_plugin_manager::plugin_manager::PluginManager;
use url::Url;

use common::message::Pagination;

use crate::{
    Context, api,
    message::{ApiError, FromService, Message},
};
pub use cosmox_plugin_manager::{
    plugin_manager::{PluginError, PluginQueryItem},
    types::PluginName,
};

#[derive(Debug, Serialize, Deserialize)]
pub struct InstallPluginParams {
    pub url: Option<Url>,
}

#[page_helper]
#[derive(Debug, Deserialize)]
pub struct PluginQueryRequest {
    /// Filter by plugin name (fuzzy match).
    pub name: Option<String>,
    /// Filter by status: "enabled", "disabled", "invalid", or empty for all.
    pub status: Option<String>,
    /// Filter by plugin type: "builtin", "external", or empty for all.
    pub plugin_type: Option<String>,
}

pub async fn install_plugin<S, E>(
    ctx: &mut Context<'_>,
    params: InstallPluginParams,
    payload: S,
) -> Result<Message<()>, ApiError<PluginError>>
where
    S: StreamExt<Item = Result<Bytes, E>> + Unpin,
    E: std::fmt::Display,
{
    ctx.access_ctx.endpoint = api::Endpoint::InstallPlugin;
    if let Some(url) = params.url {
        Message::from_service(ctx, PluginManager::install_plugin_from_url(url)).await
    } else {
        Message::from_service(ctx, PluginManager::install_plugin_from_stream(payload)).await
    }
}

pub async fn uninstall_plugin(ctx: &mut Context<'_>) -> Result<(), PluginError> {
    ctx.access_ctx.endpoint = api::Endpoint::UninstallPlugin;
    unimplemented!("Uninstall plugin api is not yet implemented.")
}

pub async fn enable_plugin(
    ctx: &mut Context<'_>,
    plugin: PluginName,
) -> Result<Message<()>, ApiError<PluginError>> {
    ctx.access_ctx.endpoint = api::Endpoint::EnablePlugin;
    Message::from_service(ctx, PluginManager::enable(plugin)).await
}

pub async fn disable_plugin(
    ctx: &mut Context<'_>,
    plugin: PluginName,
) -> Result<Message<()>, ApiError<PluginError>> {
    ctx.access_ctx.endpoint = api::Endpoint::DisablePlugin;
    Message::from_service(ctx, PluginManager::disable(plugin)).await
}

pub async fn info(ctx: &mut Context<'_>) -> Result<Message<String>, ApiError<PluginError>> {
    ctx.access_ctx.endpoint = api::Endpoint::PluginInfo;
    Message::from_service(ctx, async {
        let info = {
            #[cfg(debug_assertions)]
            {
                format!("{:#?}", PluginManager::get_plugin_manager())
            }
            #[cfg(not(debug_assertions))]
            {
                "".to_string()
            }
        };
        Ok(info)
    })
    .await
}

/// Query plugins with optional filters and pagination.
pub async fn query(
    ctx: &mut Context<'_>,
    params: PluginQueryRequest,
) -> Result<Message<Vec<PluginQueryItem>>, ApiError<PluginError>> {
    ctx.access_ctx.endpoint = api::Endpoint::QueryPlugin;
    Message::<Vec<PluginQueryItem>>::from_service(ctx, async {
        let all = PluginManager::query_plugins();

        let filtered: Vec<PluginQueryItem> = all
            .into_iter()
            .filter(|item| {
                if let Some(ref name_filter) = params.name
                    && !item.name.contains(name_filter)
                {
                    return false;
                }
                if let Some(ref status) = params.status {
                    let status = status.to_lowercase();
                    let matches = match status.as_str() {
                        "enabled" => item.enabled,
                        "disabled" => !item.enabled && item.plugin_type != "invalid",
                        "invalid" => item.plugin_type == "invalid",
                        _ => true,
                    };
                    if !matches {
                        return false;
                    }
                }
                if let Some(ref pt) = params.plugin_type {
                    let pt = pt.to_lowercase();
                    if item.plugin_type != pt {
                        return false;
                    }
                }
                true
            })
            .collect();

        let total = filtered.len() as u64;
        let page = params.page.unwrap_or(1);
        let page_size = if params.page_size == 0 {
            common::default_constants::default_page_size()
        } else {
            params.page_size
        };

        let start = ((page.saturating_sub(1)) * page_size) as usize;
        let paged: Vec<PluginQueryItem> = filtered
            .into_iter()
            .skip(start)
            .take(page_size as usize)
            .collect();

        let pagination = Pagination::new(total, page_size, page, "");

        Ok((paged, pagination))
    })
    .await
}
