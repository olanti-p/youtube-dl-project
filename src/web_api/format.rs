use crate::auth::User;
use crate::env::{DownloadFormat, EnvironmentManager};
use rocket::http::Status;
use rocket::serde::json::Json;
use rocket::{get, State};
use std::sync::Arc;

#[get("/formats")]
pub async fn get_formats(
    env: &State<Arc<EnvironmentManager>>,
    _user: User,
) -> (Status, Json<&[DownloadFormat]>) {
    let formats = env.ytdlp.get_all_formats();
    (Status::Ok, Json(formats))
}
