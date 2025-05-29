use rocket::data::{Data, ToByteUnit};
use rocket::http::{
    Method::{Get, Post},
    Status,
};
use rocket::outcome::{IntoOutcome, try_outcome};
use rocket::response::{Responder, status::Custom};
use rocket::tokio::fs::File;
use rocket::{Catcher, Request, Route, catcher, route};

fn forward<'r>(_req: &'r Request, data: Data<'r>) -> route::BoxFuture<'r> {
    Box::pin(async move { route::Outcome::forward(data, Status::NotFound) })
}

fn hi<'r>(req: &'r Request, _: Data<'r>) -> route::BoxFuture<'r> {
    route::Outcome::from(req, "Hello!").pin()
}

fn not_found_handler<'r>(_: Status, req: &'r Request) -> catcher::BoxFuture<'r> {
    let responder = Custom(Status::NotFound, format!("Couldn't find: {}", req.uri()));
    Box::pin(async move { responder.respond_to(req) })
}

#[rocket::launch]
fn rocket() -> _ {
    let always_forward = Route::ranked(1, Get, "/", forward);
    let hello = Route::ranked(2, Get, "/", hi);

    let not_found_catcher = Catcher::new(404, not_found_handler);

    // let echo = Route::new(Get, "/echo/<str>", echo_url);
    // let name = Route::new(Get, "/<name>", name);
    // let post_upload = Route::new(Post, "/", upload);
    // let get_upload = Route::new(Get, "/", get_upload);

    rocket::build()
        .mount("/", vec![always_forward, hello])
        .register("/", vec![not_found_catcher])
}
