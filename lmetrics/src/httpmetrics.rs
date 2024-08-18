use rocket::{fairing::AdHoc, http::StatusClass};

use crate::metrics;

metrics! {
pub counter http_errors_total("Amount of total http errors",
    [method, status_code]);
pub counter http_req_total("Amount of total http requests",
    [method]);
}

pub fn http_errors_metrics() -> AdHoc {
    AdHoc::on_response("response metrics", |req, res| {
        Box::pin(async move {
            let class = res.status().class();
            let meth = req.method().to_string();
            http_req_total::inc(&meth);
            if class == StatusClass::ClientError || class == StatusClass::ServerError {
                http_errors_total::inc(&meth, &res.status().code.to_string());
            }
        })
    })
}
