use rocket::get;
use rocket::response::Redirect;

#[get("/")]
pub async fn root_redirect() -> Redirect {
    Redirect::to("index_034fc9bf-de62-4470-89f1-eccef1aac743.html")
}

#[get("/index.html")]
pub async fn index_html_redirect() -> Redirect {
    Redirect::to("index_034fc9bf-de62-4470-89f1-eccef1aac743.html")
}
