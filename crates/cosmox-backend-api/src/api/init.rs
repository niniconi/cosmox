use cosmox_backend_data::services::init_service::{self, Status};
use cosmox_configuration::Configuration;
use std::{convert::Infallible, sync::atomic::Ordering};

use crate::{
    Context, api,
    message::{ApiError, FromService, Message},
};

pub use cosmox_backend_data::services::init_service::InitializeConfig;

pub async fn initialize(
    ctx: &mut Context<'_>,
    payload: InitializeConfig,
) -> Result<Message<Status>, ApiError<Infallible>> {
    ctx.access_ctx.endpoint = api::Endpoint::Init;
    Message::from_service(ctx, async move {
        let is_initialized = init_service::initialize(payload).await;
        if is_initialized {
            Configuration::get_global_configuration()
                .state
                .is_first_boot
                .store(false, Ordering::Relaxed);
        }
        Ok(Status {
            initialized: is_initialized,
        })
    })
    .await
}
