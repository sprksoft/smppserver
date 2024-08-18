use prometheus::TextEncoder;

use crate::LMetrics;
use rocket::{
    http::Method,
    route::{Handler, Outcome},
    Request, Route,
};

pub use rocket_prometheus;

#[rocket::async_trait]
impl<F: Fn() + Send + Sync + Clone + 'static> Handler for LMetrics<F> {
    async fn handle<'r>(&self, req: &'r Request<'_>, _: rocket::Data<'r>) -> Outcome<'r> {
        self.before_handle.as_ref().map(|e| e());
        let encoder = TextEncoder::new();
        let mut buf = String::new();
        encoder
            .encode_utf8(&self.registry.gather(), &mut buf)
            .expect("Failed to encode metrics");
        Outcome::from(req, buf)
    }
}
impl<F: Fn() + Send + Sync + Clone + 'static> From<LMetrics<F>> for Vec<Route> {
    fn from(other: LMetrics<F>) -> Self {
        vec![Route::new(Method::Get, "/", other)]
    }
}
