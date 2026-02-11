use std::future::{Ready, ready};

use actix_web::{
  Error,
  dev::{Service, ServiceRequest, ServiceResponse, Transform, forward_ready},
  web::Data,
};
use futures_util::future::LocalBoxFuture;
use sea_orm::DatabaseConnection;

use crate::user::security::policy_service::PolicyService;

pub struct TokenAuth;

impl<S, B> Transform<S, ServiceRequest> for TokenAuth
where
  S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
  S::Future: 'static,
  B: 'static,
{
  type Response = ServiceResponse<B>;
  type Error = Error;
  type InitError = ();
  type Transform = TokenAuthMiddleware<S>;
  type Future = Ready<Result<Self::Transform, Self::InitError>>;

  fn new_transform(&self, service: S) -> Self::Future {
    ready(Ok(TokenAuthMiddleware { service }))
  }
}

pub struct TokenAuthMiddleware<S> {
  service: S,
}

impl<S, B> Service<ServiceRequest> for TokenAuthMiddleware<S>
where
  S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
  S::Future: 'static,
  B: 'static,
{
  type Response = ServiceResponse<B>;
  type Error = Error;
  type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

  forward_ready!(service);

  fn call(&self, req: ServiceRequest) -> Self::Future {
    let auth_service = req.app_data::<Data<PolicyService>>().unwrap().clone();
    let db = req.app_data::<Data<DatabaseConnection>>().unwrap().clone();
    let path = req.path().to_string();
    let method = req.method().clone();
    let token = req.headers().get("Authorization").cloned();

    let fut = self.service.call(req);

    Box::pin(async move {
      if let Err(error) = auth_service
        .check_resource_access(token, path, method, db.clone().into_inner())
        .await
      {
        Err(actix_web::error::ErrorUnauthorized(format!("{error}")))
      } else {
        let res = fut.await?;

        Ok(res)
      }
    })
  }
}
