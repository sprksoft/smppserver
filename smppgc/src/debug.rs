use rocket::{fairing::AdHoc, get, response::Redirect, routes};

#[get("/reload_js")]
fn reload_js() -> Redirect {
    std::process::Command::new("smppgc/gen_js.sh")
        .spawn()
        .unwrap();
    Redirect::temporary("/v1")
}

pub fn stage() -> AdHoc {
    AdHoc::on_ignite("debug", |r| async { r.mount("/debug", routes![reload_js]) })
}
