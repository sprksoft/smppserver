use rocket::{
    error,
    fairing::{self, AdHoc},
    response, Build, Rocket,
};
use rocket_db_pools::Database;

pub type Result<T, E = response::Debug<sqlx::Error>> = std::result::Result<T, E>;

#[derive(Database)]
#[database("sqlx")]
pub struct Db(sqlx::PgPool);

async fn run_migrations(rocket: Rocket<Build>) -> fairing::Result {
    match Db::fetch(&rocket) {
        Some(db) => match sqlx::migrate!("../migrations").run(&**db).await {
            Ok(_) => Ok(rocket),
            Err(e) => {
                error!("Failed to initialize sqlx database: {}", e);
                Err(rocket)
            }
        },
        None => Err(rocket),
    }
}
pub fn stage() -> AdHoc {
    AdHoc::on_ignite("db", |r| async {
        r.attach(Db::init())
            .attach(AdHoc::try_on_ignite("db migrations", run_migrations))
    })
}
