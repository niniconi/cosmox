use cosmox_macros::page_helper;
use serde::{Deserialize, Serialize};

use cosmox_plugin_manager::plugin_manager::PluginManager;

use crate::{
    Context, api,
    message::{ApiError, FromService, Message},
};
pub use cosmox_plugin_manager::plugin_manager::PluginError;

#[derive(Debug, Serialize, Deserialize)]
pub struct InstallPluginParams {
    pub url: Option<String>,
}

#[page_helper]
#[derive(Debug, Deserialize)]
pub struct PluginQueryRequest {}

pub async fn install_plugin(
    ctx: &mut Context<'_>,
    payload: InstallPluginParams,
) -> Result<(), PluginError> {
    ctx.access_ctx.endpoint = api::Endpoint::InstallPlugin;
    unimplemented!("Install plugin api is not yet implemented. payload = {payload:#?}")
}

pub async fn uninstall_plugin(ctx: &mut Context<'_>) -> Result<(), PluginError> {
    ctx.access_ctx.endpoint = api::Endpoint::UninstallPlugin;
    unimplemented!("Uninstall plugin api is not yet implemented.")
}

pub async fn enable_plugin(ctx: &mut Context<'_>) -> Result<(), PluginError> {
    ctx.access_ctx.endpoint = api::Endpoint::EnablePlugin;
    unimplemented!("Enable plugin api is not yet implemented.")
}

pub async fn disable_plugin(ctx: &mut Context<'_>) -> Result<(), PluginError> {
    ctx.access_ctx.endpoint = api::Endpoint::DisablePlugin;
    unimplemented!("Disable plugin api is not yet implemented.")
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
