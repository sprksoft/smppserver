use rocket::{fairing::AdHoc, response};
use rocket_db_pools::{deadpool_redis, Database};

#[derive(Database)]
#[database("redis")]
pub struct Db(deadpool_redis::Pool);

pub fn stage() -> AdHoc {
    AdHoc::on_ignite("db", |r| async { r.attach(Db::init()) })
}
