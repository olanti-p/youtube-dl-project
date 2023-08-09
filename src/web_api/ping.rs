use rocket::http::Status;
use rocket::post;

#[post("/ping")]
pub async fn pong() -> (Status, &'static str) {
    (Status::Ok, "Pong!")
}
