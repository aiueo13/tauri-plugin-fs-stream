use std::borrow::Cow;
use serde::{ser::Serializer, Serialize};

pub type Result<T> = std::result::Result<T, Error>;


#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("{0}")]
    Raw(Cow<'static, str>),

    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error(transparent)]
    Utf8(#[from] std::str::Utf8Error),

    #[error(transparent)]
    ParseInt(#[from] std::num::ParseIntError),

    #[error(transparent)]
    Tauri(#[from] tauri::Error),

    #[error(transparent)]
    TauriHttpHeaderToStr(#[from] tauri::http::header::ToStrError),

    #[error(transparent)]
    SerdeJson(#[from] serde_json::Error),

    #[error(transparent)]
    Base64Decode(#[from] base64::DecodeError),
}

#[allow(unused)]
impl Error {

    pub fn with(msg: impl Into<Cow<'static, str>>) -> Self {
        Self::Raw(msg.into())
    }

    pub fn missing_value(value_name: impl std::fmt::Display) -> Self {
        Self::with(format!("missing value: {value_name}"))
    }

    pub fn invalid_type(value_name: impl std::fmt::Display) -> Self {
        Self::with(format!("invalid type for {value_name}"))
    }

    pub fn invalid_value(value_name: impl std::fmt::Display, value: impl std::fmt::Display) -> Self {
        Self::with(format!("invalid value: {value} for {value_name}"))
    }
}


impl<T> From<std::sync::PoisonError<T>> for Error {

    fn from(_: std::sync::PoisonError<T>) -> Error {
        Error::with("thread poisoned")
    }
}

impl Serialize for Error {

    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            Error::Raw(msg) => serializer.serialize_str(msg),
            e => serializer.serialize_str(&e.to_string()),
        }
    }
}