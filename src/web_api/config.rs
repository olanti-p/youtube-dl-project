use crate::auth::User;
use crate::env::{Config, EnvironmentManager};
use crate::exit_status::{ExitStatus, ExitStatusHandle};
use rocket::form::Form;
use rocket::http::Status;
use rocket::serde::json::Json;
use rocket::serde::{Deserialize, Serialize};
use rocket::{get, post, FromForm, Shutdown, State};
use std::sync::Arc;
use tracing::warn;

#[derive(Debug, Serialize, Deserialize, FromForm)]
pub struct NewConfigForm {
    pub value: String,
}

#[get("/config")]
pub async fn get_config(
    env: &State<Arc<EnvironmentManager>>,
    _user: User,
) -> (Status, Json<Config>) {
    (Status::Ok, Json(env.config.clone()))
}

#[post("/config", data = "<data>")]
pub async fn set_config(
    exit_status: &State<ExitStatusHandle>,
    shutdown: Shutdown,
    _user: User,
    data: Form<NewConfigForm>,
) -> Status {
    match serde_json::from_str::<Config>(&data.value) {
        Ok(new_config) => {
            if EnvironmentManager::check_config_validity(&new_config).await {
                warn!("Preparing for config update, requesting shutdown...");
                exit_status
                    .store(ExitStatus::ChangeConfig(new_config))
                    .await;
                shutdown.notify();
                Status::Accepted
            } else {
                {
                    warn!("New config failed validation: {:?}", new_config);
                    Status::BadRequest
                }
            }
        }
        Err(e) => {
            warn!("Failed to parse proposed config: {e} {:?}", data.value);
            Status::BadRequest
        }
    }
}
