use crate::auth::user::User;

#[derive(Debug)]
pub enum AuthRequestState {
    None,
    Error(anyhow::Error),
    Authorized(User),
}
