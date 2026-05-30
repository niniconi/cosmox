pub mod controller;
pub mod io;
pub mod message;
pub mod middleware;

use std::{error::Error, sync::Arc};

use actix_cors::Cors;
use actix_web::{App, HttpServer, dev::ServerHandle, http::header, web};
use common::Handle;
use cosmox_configuration::Configuration;

use crate::{
    controller::{
        docs_controller, init, library_controller, metadata_controller, path_tree_scanner,
        plugin_controller, resource_controller, role_permission_controller, scanner_controller,
        search_controller, system_controller, tag_controller, ui_controller, user_controller,
    },
    io::transfer_controller,
    middleware::auth_middleware::TokenExtractor,
};

pub struct RawServerHandle(ServerHandle);
impl Handle for RawServerHandle {
    async fn stop(&mut self, graceful: bool) {
        self.0.stop(graceful).await;
    }
}

pub fn server(
    config: &'static Configuration,
    host: &'static str,
    port: u16,
) -> Result<(impl Future<Output = Result<(), impl Error>>, impl Handle), std::io::Error> {
    let server = HttpServer::new(move || {
        let cors = Cors::default()
            .allow_any_origin()
            .allowed_methods(vec!["GET", "POST", "PUT", "PATCH", "DELETE", "OPTIONS"])
            .allowed_headers(vec![
                header::AUTHORIZATION,
                header::ACCEPT,
                header::CONTENT_TYPE,
                header::RANGE,
                header::IF_RANGE,
                header::CACHE_CONTROL,
                header::PRAGMA,
                header::HeaderName::from_static("x-session-id"),
                header::HeaderName::from_static("x-client-version"),
            ])
            .expose_headers(vec![
                header::CONTENT_RANGE,
                header::CONTENT_LENGTH,
                header::ACCEPT_RANGES,
                header::SERVER,
            ])
            .max_age(3600);

        App::new()
            .app_data(web::Data::new(Arc::new(config)))
            .wrap(cors)
            .wrap(TokenExtractor)
            .service(
                web::scope("/api")
                    .service(init::initialize)
                    .service(search_controller::search)
                    .service(
                        web::scope("/system")
                            .service(system_controller::info)
                            .service(system_controller::restart)
                            .service(system_controller::shutdown)
                            .service(system_controller::about)
                            .service(system_controller::log)
                            .service(system_controller::delete_all),
                    )
                    .service(
                        web::scope("/library")
                            .service(library_controller::modify)
                            .service(library_controller::add)
                            .service(library_controller::delete)
                            .service(library_controller::query)
                            .service(library_controller::get_all_types)
                            .service(library_controller::get),
                    )
                    .service(
                        web::scope("/tag")
                            .service(tag_controller::add)
                            .service(tag_controller::add_group)
                            .service(tag_controller::delete_group)
                            .service(tag_controller::query)
                            .service(tag_controller::catalog)
                            .service(tag_controller::query_group)
                            .service(tag_controller::get_group)
                            .service(tag_controller::get),
                    )
                    .service(
                        web::scope("/resource")
                            .service(resource_controller::add)
                            .service(resource_controller::delete)
                            .service(resource_controller::get_metadata)
                            .service(resource_controller::query)
                            .service(resource_controller::add_tag)
                            .service(resource_controller::get),
                    )
                    .service(
                        web::scope("/user")
                            .service(user_controller::login)
                            .service(user_controller::register)
                            .service(user_controller::delete)
                            .service(user_controller::query)
                            .service(user_controller::upload_avatar)
                            .service(user_controller::add_role)
                            .service(user_controller::get)
                            .service(
                                web::scope("/acl")
                                    .service(role_permission_controller::add_role)
                                    .service(role_permission_controller::delete_role)
                                    .service(role_permission_controller::add_permission)
                                    .service(role_permission_controller::delete_permission)
                                    .service(role_permission_controller::add_permission_for_role)
                                    .service(role_permission_controller::query_role)
                                    .service(role_permission_controller::query_permission),
                            ),
                    )
                    .service(
                        web::scope("/item")
                            .service(transfer_controller::pull)
                            .service(transfer_controller::push),
                    )
                    .service(
                        web::scope("/scanner")
                            .service(scanner_controller::scan_all)
                            .service(scanner_controller::scan)
                            .service(scanner_controller::get_status)
                            .service(scanner_controller::add_task)
                            .service(scanner_controller::info)
                            .service(path_tree_scanner::get_sub_path),
                    )
                    .service(
                        web::scope("/plugin")
                            .service(plugin_controller::install_plugin)
                            .service(plugin_controller::uninstall_plugin)
                            .service(plugin_controller::enable_plugin)
                            .service(plugin_controller::disable_plugin)
                            .service(plugin_controller::info),
                    )
                    .service(
                        web::scope("/metadata")
                            .service(metadata_controller::query)
                            .service(metadata_controller::get),
                    )
                    .service(web::scope("/ui").service(ui_controller::get_core))
                    .service(docs_controller::openapi_yaml)
                    .service(docs_controller::scalar_docs),
            )
            .service(actix_files::Files::new("/", "./static").index_file("index.html"))
    })
    .disable_signals()
    .bind((host, port))?
    .run();
    let handle = RawServerHandle(server.handle());
    Ok((server, handle))
}
