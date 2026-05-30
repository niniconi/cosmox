pub mod default_constants;
pub mod fs;
pub mod message;
pub mod security;

pub trait Handle {
    fn stop(&mut self, graceful: bool) -> impl Future<Output = ()> + Send;
}
