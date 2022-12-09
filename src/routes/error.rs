use rocket::{request::FlashMessage, Route};
use rocket_dyn_templates::{context, Template};

#[must_use]
pub fn routes() -> Vec<Route> {
    routes![server_error_panel]
}

#[get("/servererror")]
pub fn server_error_panel(flash: FlashMessage<'_>) -> Template {
    Template::render(
        "server-error",
        context! {
            error: flash.message().to_string(),
        },
    )
}
