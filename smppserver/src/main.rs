use rocket::launch;

mod globalchat;

#[launch]
fn rocket() -> _ {
    rocket::build().attach(globalchat::stage())
}
