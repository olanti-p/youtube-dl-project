use crate::auth::User;
use rocket::http::Status;
use rocket::{post, Shutdown};

#[post("/shutdown_server")]
pub async fn shutdown_server(shutdown: Shutdown, _user: User) -> (Status, &'static str) {
    shutdown.notify();
    (Status::Accepted, "Shutting down...")
}
