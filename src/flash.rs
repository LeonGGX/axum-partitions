// flash.rs

use axum::http::{header, HeaderMap, HeaderValue, StatusCode};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use tower_cookies::{Cookie, Cookies};

#[derive(Deserialize)]
struct ValuedMessage<T> {
    #[serde(rename = "_")]
    value: T,
}

#[derive(Serialize)]
struct ValuedMessageRef<'a, T> {
    #[serde(rename = "_")]
    value: &'a T,
}

const FLASH_COOKIE_NAME: &str = "_flash";

pub fn get_flash_cookie<T>(cookies: &Cookies) -> Option<T>
    where
        T: DeserializeOwned,
{
    cookies.get(FLASH_COOKIE_NAME).and_then(|flash_cookie| {
        if let Ok(ValuedMessage::<T> { value }) = serde_json::from_str(flash_cookie.value()) {
            Some(value)
        } else {
            None
        }
    })
}

pub type PersonResponse = (StatusCode, HeaderMap);
pub type GenreResponse = (StatusCode, HeaderMap);
//pub type PartitionResponse = (StatusCode, HeaderMap);

pub fn person_response<T>(cookies: &mut Cookies, data: T) -> PersonResponse
    where
        T: Serialize,
{
    let valued_message_ref = ValuedMessageRef { value: &data };

    let mut cookie = Cookie::new(
        FLASH_COOKIE_NAME,
        serde_json::to_string(&valued_message_ref).unwrap(),
    );
    cookie.set_path("/persons");
    cookies.add(cookie);

    let mut header = HeaderMap::new();
    header.insert(header::LOCATION, HeaderValue::from_static("/persons"));

    (StatusCode::SEE_OTHER, header)
}

pub fn genre_response<T>(cookies: &mut Cookies, data: T) -> GenreResponse
    where
        T: Serialize,
{
    let valued_message_ref = ValuedMessageRef { value: &data };

    let mut cookie = Cookie::new(
        FLASH_COOKIE_NAME,
        serde_json::to_string(&valued_message_ref).unwrap(),
    );
    cookie.set_path("/genres");
    cookies.add(cookie);

    let mut header = HeaderMap::new();
    header.insert(header::LOCATION, HeaderValue::from_static("/genres"));

    (StatusCode::SEE_OTHER, header)
}
