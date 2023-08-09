use crate::auth::auth_request_state::AuthRequestState;
use rand::prelude::SliceRandom;
use rand::Rng;
use rocket::http::Status;
use rocket::request::{FromRequest, Outcome};
use rocket::Request;
use sqlx::FromRow;
use uuid::Uuid;

/**
 * Disclaimer: with this simple authorization system, we only try to
 * block potentially malicious software from submitting download requests
 * of potentially malicious links.
 *
 * There is no protection from local interference (the password is
 * stored in the database as plain text and is available via command,
 * connection is plain HTTP rather then HTTPS) because if hacker has
 * gained enough privileges to read local files or monitor web traffic
 * on the machine, they don't need to hack our server anymore.
 *
 * There's no protection on the webui page either because it can only
 * be modified through a malicious extension, and if the user has
 * installed such extension they're most likely already screwed.
 */

#[derive(Debug, Default, Clone, FromRow)]
pub struct User {
    user_id: Uuid,
    #[allow(dead_code)]
    name: String,
    api_token: String,
}

const TOKEN_CHARACTERS: &[u8; 62] =
    b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";

impl User {
    pub fn admin_user_name() -> &'static str {
        "admin"
    }

    pub fn generate_api_token<R: Rng + ?Sized>(rng: &mut R) -> String {
        (0..32)
            .map(|_| *TOKEN_CHARACTERS.choose(rng).unwrap() as char)
            .collect()
    }

    pub fn get_user_id(&self) -> Uuid {
        self.user_id
    }

    pub fn get_api_token(&self) -> &str {
        &self.api_token
    }
}

#[rocket::async_trait]
impl<'r> FromRequest<'r> for User {
    type Error = anyhow::Error;

    async fn from_request(req: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        let auth = req.local_cache(|| AuthRequestState::None);
        match auth {
            AuthRequestState::None => Outcome::Forward(()),
            AuthRequestState::Authorized(user) => Outcome::Success(user.clone()),
            AuthRequestState::Error(_) => {
                Outcome::Failure((Status::Unauthorized, anyhow!("unarthorized")))
            }
        }
    }
}
