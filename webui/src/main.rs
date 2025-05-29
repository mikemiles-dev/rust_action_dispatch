use rocket::data::{Data, ToByteUnit};
use rocket::http::{
    Method::{Get, Post},
    Status,
};
use rocket::outcome::{IntoOutcome, try_outcome};
use rocket::response::Redirect;
use rocket::response::{Responder, status::Custom};
use rocket::routes;
use rocket::tokio::fs::File;
use rocket::{Catcher, Request, Route, catcher, route};
use rocket::{get, uri};

use rocket_dyn_templates::{
    Template, context,
    minijinja::{self, Environment},
};

fn forward<'r>(_req: &'r Request, data: Data<'r>) -> route::BoxFuture<'r> {
    Box::pin(async move { route::Outcome::forward(data, Status::NotFound) })
}

// fn hi<'r>(req: &'r Request, _: Data<'r>) -> route::BoxFuture<'r> {
//     route::Outcome::from(req, "Hello!").pin()
// }

#[get("/hello/<name>")]
pub fn hello(name: &str) -> Template {
    Template::render(
        "/index",
        context! {
            title: "Hello",
            name: Some(name),
            items: vec!["One", "Two", "Three"],
        },
    )
}

#[get("/")]
pub fn index() -> Redirect {
    Redirect::to(uri!("/", hello(name = "Your Name")))
}

fn not_found_handler<'r>(_: Status, req: &'r Request) -> catcher::BoxFuture<'r> {
    let responder = Custom(Status::NotFound, format!("Couldn't find: {}", req.uri()));
    Box::pin(async move { responder.respond_to(req) })
}

pub fn customize(env: &mut Environment) {
    env.add_template(
        "minijinja/about.html",
        r#"
        {% extends "minijinja/layout" %}

        {% block page %}
            <section id="about">
                <h1>About - Here's another page!</h1>
            </section>
        {% endblock %}
    "#,
    )
    .expect("valid Jinja2 template");
}

#[rocket::launch]
fn rocket() -> _ {
    let not_found_catcher = Catcher::new(404, not_found_handler);

    // let echo = Route::new(Get, "/echo/<str>", echo_url);
    // let name = Route::new(Get, "/<name>", name);
    // let post_upload = Route::new(Post, "/", upload);
    // let get_upload = Route::new(Get, "/", get_upload);

    rocket::build()
        .mount("/", routes![index, hello])
        .register("/", vec![not_found_catcher])
        .attach(Template::custom(|engines| {
            customize(&mut engines.minijinja);
        }))
}
