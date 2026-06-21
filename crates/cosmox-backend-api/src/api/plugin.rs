use bytes::Bytes;
use cosmox_macros::page_helper;
use futures_util::StreamExt;
use serde::{Deserialize, Serialize};

use cosmox_plugin_manager::plugin_manager::PluginManager;
use url::Url;

use crate::{
    Context, api,
    message::{ApiError, FromService, Message},
};
pub use cosmox_plugin_manager::{plugin_manager::PluginError, types::PluginName};

#[derive(Debug, Serialize, Deserialize)]
pub struct InstallPluginParams {
    pub url: Option<Url>,
}

#[page_helper]
#[derive(Debug, Deserialize)]
pub struct PluginQueryRequest {}

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
