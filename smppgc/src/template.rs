use rocket::{
    fairing::AdHoc,
    get,
    http::{Header, Status},
    request::{self, FromRequest},
    response::Responder,
    routes, Request, State,
};
use rocket_dyn_templates::{context, Template};

use crate::{ListenAddress, OfflineConfig};

macro_rules! theme {
    ($vis:vis $name:ident{$($param:ident:$default_value:literal),*}) => {
        $vis struct $name {
            pub $($param:String),*
        }
        impl $name {
            pub fn css(&self) -> String {
                format!(concat!("body{{", $(concat!("--", stringify!($param), ": #{};")),*, "}}"), $(self.$param),*)
            }
        }

        #[rocket::async_trait]
        impl<'r> FromRequest<'r> for $name {
            type Error = ();

            async fn from_request(req: &'r Request<'_>) -> request::Outcome<Self, Self::Error> {
                $(
                    let $param = match req
                    .query_value::<String>(stringify!($param))
                    .unwrap_or(Ok($default_value.to_string()))
                    {
                        Ok(value) => {
                            if value.len() > 8{
                                return request::Outcome::Error((Status::BadRequest, ()));
                            }
                            for character in value.chars(){
                                if !character.is_alphanumeric(){
                                    return request::Outcome::Error((Status::BadRequest, ()));
                                }
                            }
                            value
                        },
                        Err(_) => return request::Outcome::Error((Status::BadRequest, ())),
                    };
                )*
                request::Outcome::Success($name {
                    $($param),*
                })
            }
        }
    };
}

theme! {
    SmppTheme{
        color_text:"c2bab2",
        color_base00:"191817",
        color_base01:"232020",
        color_base02:"2b2828",
        color_base03:"353232",
        color_base04:"3f3c3c",
        color_base05:"4a4747",
        color_accent:"ffd5a0"
    }
}

struct CSPFrameAncestors {
    frame_ancestors: String,
}
impl From<CSPFrameAncestors> for Header<'static> {
    fn from(csp: CSPFrameAncestors) -> Self {
        Header::new(
            "Content-Security-Policy",
            format!("frame-ancestors {};", csp.frame_ancestors),
        )
    }
}

struct XFrameOptions {
    allow_from: String,
}
impl From<XFrameOptions> for Header<'static> {
    fn from(xfo: XFrameOptions) -> Self {
        Header::new("X-Frame-Options", format!("ALLOW-FROM {}", xfo.allow_from))
    }
}

#[derive(Responder)]
enum GcPageResponder {
    #[response(status = 200)]
    Ok {
        inner: Template,
        csp: CSPFrameAncestors,
        xfo: XFrameOptions,
    },
    #[response(status = 400)]
    BadRequest(&'static str),
}

#[get("/v1?<skip_login>&<placeholder>")]
fn v1(
    theme: SmppTheme,
    placeholder: Option<&str>,
    skip_login: Option<bool>,
    offline_config: &State<OfflineConfig>,
    listen_address: &State<ListenAddress>,
) -> GcPageResponder {
    let placeholder = placeholder.unwrap_or("");
    if placeholder.contains(['<', '>', '=', '"', '"']) {
        return GcPageResponder::BadRequest("xss detected");
    }

    let debug = cfg!(debug_assertions);
    let root_url = if debug {
        format!("://{}", listen_address.listen_address)
    } else {
        "s://ldev.eu.org/smpp/gc".to_string()
    };
    GcPageResponder::Ok {
        inner: Template::render(
            "v1",
            context! {theme_css:theme.css(), placeholder:placeholder, root_url: root_url, debug: debug, offline: offline_config.offline, skip_login:skip_login.unwrap_or(false), version: env!("CARGO_PKG_VERSION")},
        ),
        csp: CSPFrameAncestors {
            frame_ancestors: "*.smartschool.be".to_string(),
        },
        xfo: XFrameOptions {
            allow_from: "*.smartschool.be".to_string(),
        },
    }
}

pub fn stage() -> AdHoc {
    AdHoc::on_ignite("templates", |r| async {
        r.mount("/", routes![v1])
            .attach(Template::custom(|engines| {
                let hdb = &mut engines.handlebars;
                hdb.set_strict_mode(true);
                #[cfg(debug_assertions)]
                hdb.set_dev_mode(true);
            }))
    })
}
