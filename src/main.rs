// main.rs

mod flash;
mod persons;
mod genre;
mod partition;

use axum::{
    extract::{Form, Extension, Path,},
    http::{StatusCode, Uri,},
    response::{Html},
    routing::{get, post, get_service},
    Router,
    AddExtensionLayer,
};

use serde::{Deserialize, Serialize};
use std::{env, net::SocketAddr};
use std::str::FromStr;
use axum::extract::Query;

use tera::Tera;

use tokio::signal;

use tower::ServiceBuilder;
use tower_cookies::{CookieManagerLayer, Cookies,};
use tower_http::services::ServeDir;

use persons::Entity as Person;
use genre::Entity as Genre;
use sea_orm::{prelude::*, Database, Set, Order, QueryOrder};

use crate::flash::{
    get_flash_cookie,
    person_response,
    PersonResponse,
    genre_response,
    GenreResponse
};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Set the RUST_LOG, if it hasn't been explicitly defined
    if std::env::var_os("RUST_LOG").is_none() {
        std::env::set_var("RUST_LOG", "axum_jwt=debug")
    }
    tracing_subscriber::fmt::init();

    env::set_var( "JWT_SECRET", "secret");

    dotenv::dotenv().ok();
    let db_url = env::var("DATABASE_URL").expect("DATABASE_URL is not set in .env file");
    let host = env::var("HOST").expect("HOST is not set in .env file");
    let port = env::var("PORT").expect("PORT is not set in .env file");
    let server_url = format!("{}:{}", host, port);

    let conn = Database::connect(db_url)
        .await
        .expect("Database connection failed");

    let templates = Tera::new(concat!(env!("CARGO_MANIFEST_DIR"), "/templates/**/*"))
        .expect("Tera initialization failed");

    let app = Router::new()
        .fallback(get(handler_404))
        .route("/", get(root))
        .route("/about", get(about))

        .route("/persons", get(list_persons))
        .route("/persons/add", post(create_person))
        .route("/persons/:id", post(update_person))
        .route("/persons/delete/:id", post(delete_person))
        .route("/persons/print", get(print_list_persons))
        .route("/persons/find", post(find_person_by_name))

        .route("/genres", get(list_genres))
        .route("/genres/add", post(create_genre))
        .route("/genres/:id", post(update_genre))
        .route("/genres/delete/:id", post(delete_genre))
        .route("/genres/print", get(print_list_genres))
        .route("/genres/find", post(find_genre_by_name))

        .nest(
            "/static",
                get_service(ServeDir::new(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/static"
                )))
                    .handle_error(|error: std::io::Error| async move {
                        (
                            StatusCode::INTERNAL_SERVER_ERROR,
                            format!("Unhandled internal error: {}", error),
                        )
                    }),
        )
        .layer(
            ServiceBuilder::new()
                .layer(CookieManagerLayer::new())
                .layer(AddExtensionLayer::new(conn))
                .layer(AddExtensionLayer::new(templates)));

    let addr = SocketAddr::from_str(&server_url).unwrap();
    tracing::debug!("listening on {}", addr);

    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    Ok(())
}

#[derive(Deserialize, Serialize, Debug, Clone)]
struct FlashData {
    kind: String,
    message: String,
}
// Il faut une fonction root qui ramène à la racine
// sinon problème. Sauf si on utilise Redirect
//
async fn root(Extension(ref templates): Extension<Tera>,) -> Result<Html<String>, (StatusCode, &'static str)> {
    let title = "Start";
    let mut ctx = tera::Context::new();
    ctx.insert("title", &title);
    let body = templates
        .render("start.html.tera", &ctx)
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Template error"))?;

    Ok(Html(body))
}

async fn about(
    Extension(ref templates): Extension<Tera>,
) -> Result<Html<String>, (StatusCode, &'static str)> {
    let title = "A propos de ...";
    let mut ctx = tera::Context::new();
    ctx.insert("title", &title);
    let body = templates
        .render("about.html.tera", &ctx)
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Template error"))?;

    Ok(Html(body))
}


async fn handler_404(
    Extension(ref templates): Extension<Tera>,
    uri: Uri,
) -> Result<Html<String>, (StatusCode, &'static str)> {

    let origin = uri.path();
    //println!("uri : {}", origin);
    let mut ctx = tera::Context::new();
    ctx.insert("uri", origin);
    let body = templates
        .render("error/404.html.tera", &ctx)
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Template error in 404.html.tera"))?;

    Ok(Html(body))
}

async fn list_genres(Extension(ref templates): Extension<Tera>,
                     Extension(ref conn): Extension<DatabaseConnection>,
                     cookies: Cookies,)->  Result<Html<String>, (StatusCode, &'static str)> {

    let genres = Genre::find()
        .order_by(genre::Column::Name, Order::Asc)
        .all(conn)
        .await
        .unwrap();

    let title = "Gestion des Genres";

    let mut ctx = tera::Context::new();
    ctx.insert("title", &title);
    ctx.insert("genres", &genres);

    if let Some(value) = get_flash_cookie::<FlashData>(&cookies) {
        ctx.insert("flash", &value);
    }

    let body = templates
        .render("genres.html.tera", &ctx)
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Template error in genres.html.tera"))?;

    Ok(Html(body))
}

async fn list_persons(
    Extension(ref templates): Extension<Tera>,
    Extension(ref conn): Extension<DatabaseConnection>,
    cookies: Cookies,)->  Result<Html<String>, (StatusCode, &'static str)> {

    let persons = Person::find()
        .order_by(persons::Column::FullName, Order::Asc)
        .all(conn)
        .await
        .unwrap();
    let title = "Gestion des Musiciens";

    let mut ctx = tera::Context::new();
    ctx.insert("title", &title);
    ctx.insert("persons", &persons);

    if let Some(value) = get_flash_cookie::<FlashData>(&cookies) {
        ctx.insert("flash", &value);
    }

    let body = templates
        .render("persons.html.tera", &ctx)
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Template error in persons.html.tera"))?;

    Ok(Html(body))
}

async fn create_genre(
    Extension(ref conn): Extension<DatabaseConnection>,
    form: Form<genre::Model>,
    mut cookies: Cookies,
//)->  impl IntoResponse {
)-> Result<GenreResponse, (StatusCode, &'static str)> {
    let model = form.0;
    genre::ActiveModel {
        name: Set(model.name).to_owned(),
        ..Default::default()
    }
        .save(conn)
        .await
        .expect("could not insert genre");
    let data = FlashData {
        kind: "success".to_owned(),
        message: "Genre successfully added".to_owned(),
    };
    //Redirect::to("/genres".parse().unwrap())
    Ok(genre_response(&mut cookies, data))
}

async fn create_person(
    Extension(ref conn): Extension<DatabaseConnection>,
    form: Form<persons::Model>,
    mut cookies: Cookies,
//)->  impl IntoResponse {
)-> Result<PersonResponse, (StatusCode, &'static str)> {

    let model = form.0;
    persons::ActiveModel {
        full_name: Set(model.full_name).to_owned(),
        ..Default::default()
    }
        .save(conn)
        .await
        .expect("could not insert person");

    let data = FlashData {
        kind: "success".to_owned(),
        message: "Person successfully added".to_owned(),
    };
    // Redirect::to("/persons".parse().unwrap())
    Ok(person_response(&mut cookies, data))
}

async fn update_genre(
    Extension(ref conn): Extension<DatabaseConnection>,
    Path(id): Path<i32>,
    form: Form<genre::Model>,
    mut cookies: Cookies,
)->  Result<GenreResponse, (StatusCode, &'static str)> {
    let model = form.0;
    genre::ActiveModel {
        id: Set(id),
        name: Set(model.name.to_owned()),
    }
        .save(conn)
        .await
        .expect("could not edit genre");

    let data = FlashData {
        kind: "success".to_owned(),
        message: "Genre successfully updated".to_owned(),
    };

    Ok(genre_response(&mut cookies, data))
}

async fn update_person(
      Extension(ref conn): Extension<DatabaseConnection>,
      Path(id): Path<i32>,
      form: Form<persons::Model>,
      mut cookies: Cookies,
)->  Result<PersonResponse, (StatusCode, &'static str)> {

    let model = form.0;
    persons::ActiveModel {
        id: Set(id),
        full_name: Set(model.full_name.to_owned()),
    }
        .save(conn)
        .await
        .expect("could not edit person");

    let data = FlashData {
        kind: "success".to_owned(),
        message: "Person successfully updated".to_owned(),
    };

    Ok(person_response(&mut cookies, data))
}

async fn delete_genre(
    Extension(ref conn): Extension<DatabaseConnection>,
    Path(id): Path<i32>,
    mut cookies: Cookies,
) -> Result<PersonResponse, (StatusCode, &'static str)> {

    let genre: genre::ActiveModel = Genre::find_by_id(id)
        .one(conn)
        .await
        .unwrap()
        .unwrap()
        .into();

    genre.delete(conn).await.unwrap();

    let data = FlashData {
        kind: "success".to_owned(),
        message: "Genre succcessfully deleted".to_owned(),
    };

    Ok(genre_response(&mut cookies, data))
}

async fn delete_person(
    Extension(ref conn): Extension<DatabaseConnection>,
    Path(id): Path<i32>,
    mut cookies: Cookies,
) -> Result<PersonResponse, (StatusCode, &'static str)> {
    let person: persons::ActiveModel = Person::find_by_id(id)
        .one(conn)
        .await
        .unwrap()
        .unwrap()
        .into();

    person.delete(conn).await.unwrap();

    let data = FlashData {
        kind: "success".to_owned(),
        message: "Person succcessfully deleted".to_owned(),
    };

    Ok(person_response(&mut cookies, data))
}

async fn print_list_genres(
    Extension(ref templates): Extension<Tera>,
    Extension(ref conn): Extension<DatabaseConnection>,
    _cookies: Cookies,
)->  Result<Html<String>, (StatusCode, &'static str)> {

    let genres = Genre::find()
        .order_by(genre::Column::Name, Order::Asc)
        .all(conn)
        .await
        .unwrap();

    let title = "Liste des Genres";

    let mut ctx = tera::Context::new();
    ctx.insert("title", &title);
    ctx.insert("genres", &genres);

    let body = templates
        .render("list_genres.html.tera", &ctx)
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Template error in list_genres.html.tera"))?;

    Ok(Html(body))
}

async fn print_list_persons(
    Extension(ref templates): Extension<Tera>,
    Extension(ref conn): Extension<DatabaseConnection>,
    _cookies: Cookies,
)->  Result<Html<String>, (StatusCode, &'static str)> {
    let persons = Person::find().order_by(persons::Column::FullName, Order::Asc).all(conn).await.unwrap();
    let title = "Liste des Personnes";

    let mut ctx = tera::Context::new();
    ctx.insert("title", &title);
    ctx.insert("persons", &persons);

    let body = templates
        .render("list_users.html.tera", &ctx)
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Template error"))?;

    Ok(Html(body))
}

#[derive(Deserialize, Serialize, Debug, Clone,)]
pub struct Demande {
    pub name : String,
}

async fn find_genre_by_name(
    Extension(ref templates): Extension<Tera>,
    Extension(ref conn): Extension<DatabaseConnection>,
    form: Form<Demande>,
    _cookies: Cookies,
)->  Result<Html<String>, (StatusCode, &'static str)> {

    let demande = form.0;
    tracing::debug!("name : {:?}", demande);

    let name = demande.name;

    let genres: Vec<genre::Model> = Genre::find()
        .filter(genre::Column::Name.contains(&name))
        .all(conn).await.unwrap();

    let title = "Genre(s) trouvé(s)";

    let mut ctx = tera::Context::new();
    ctx.insert("title", &title);
    ctx.insert("genres", &genres);

    let body = templates
        .render("genres.html.tera", &ctx)
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Template error"))?;

    Ok(Html(body))
}

async fn find_person_by_name(
    Extension(ref templates): Extension<Tera>,
    Extension(ref conn): Extension<DatabaseConnection>,
    form: Form<Demande>,
    _cookies: Cookies,
)->  Result<Html<String>, (StatusCode, &'static str)> {

    let demande = form.0;
    //println!("name : {}", name);
    tracing::debug!("name : {:?}", demande);

    let name = demande.name;

    let persons: Vec<persons::Model> = Person::find()
        .filter(persons::Column::FullName.contains(&name))
        .all(conn).await.unwrap();

    let title = "Personne(s) trouvée(s)";

    let mut ctx = tera::Context::new();
    ctx.insert("title", &title);
    ctx.insert("persons", &persons);

    let body = templates
        .render("persons.html.tera", &ctx)
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Template error"))?;

    Ok(Html(body))
}

async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
        let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
        let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }

    println!("signal received, starting graceful shutdown");
    tracing::debug!("signal ctrl_c reçu");
}




