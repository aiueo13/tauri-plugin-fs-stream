use std::borrow::Cow;
use serde::{ser::Serializer, Serialize};


pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, thiserror::Error)]
#[error(transparent)]
pub struct Error {
    inner: InnerError
}

#[allow(unused)]
impl crate::Error {

    pub fn with(msg: impl Into<Cow<'static, str>>) -> Self {
        Self { inner: InnerError::Raw(msg.into()) }
    }

    pub fn missing_value(value_name: impl std::fmt::Display) -> Self {
        Self::with(format!("missing value: {value_name}"))
    }

    pub fn invalid_type(type_name: impl std::fmt::Display) -> Self {
        Self::with(format!("invalid type for {type_name}"))
    }

    pub fn invalid_value(value_name: impl std::fmt::Display, value: impl std::fmt::Display) -> Self {
        Self::with(format!("invalid value: {value} for {value_name}"))
    }
}

impl From<crate::Error> for std::io::Error {

    fn from(e: crate::Error) -> std::io::Error {
        match e.inner {
            InnerError::Io(e) => e,
            e => std::io::Error::new(std::io::ErrorKind::Other, e)
        }
    }
}

impl From<crate::Error> for tauri::Error {

    fn from(e: crate::Error) -> tauri::Error {
        match e.inner {
            InnerError::Tauri(e) => e,
            InnerError::Io(e) => tauri::Error::Io(e),

            #[cfg(target_os = "android")]
            InnerError::PluginInvoke(e) => tauri::Error::PluginInvoke(e),

            e => tauri::Error::Anyhow(e.into()),
        }
    }
}


#[derive(Debug, thiserror::Error)]
enum InnerError {
    #[error("{0}")]
    Raw(Cow<'static, str>),

    #[error(transparent)]
    Io(std::io::Error),

    #[error(transparent)]
    Utf8(std::str::Utf8Error),

    #[error(transparent)]
    ParseInt(std::num::ParseIntError),

    #[error(transparent)]
    Tauri(tauri::Error),

    #[error(transparent)]
    TauriHttpHeaderToStr(tauri::http::header::ToStrError),

    #[error(transparent)]
    SerdeJson(serde_json::Error),

    #[error(transparent)]
    Base64Decode(base64::DecodeError)
}

macro_rules! impl_into_err_from_inner {
    ($from:ty, $e:pat => $a:expr) => {
        impl From<$from> for crate::Error {
            fn from($e: $from) -> crate::Error {
                $a
            }
        }
    };
}

impl_into_err_from_inner!(std::io::Error, e => crate::Error { inner: InnerError::Io(e) });
impl_into_err_from_inner!(std::str::Utf8Error, e => crate::Error { inner: InnerError::Utf8(e) });
impl_into_err_from_inner!(std::num::ParseIntError, e => crate::Error { inner: InnerError::ParseInt(e) });
impl_into_err_from_inner!(tauri::Error, e => crate::Error { inner: InnerError::Tauri(e) });
impl_into_err_from_inner!(tauri::http::header::ToStrError, e => crate::Error { inner: InnerError::TauriHttpHeaderToStr(e) });
impl_into_err_from_inner!(serde_json::Error, e => crate::Error { inner: InnerError::SerdeJson(e) });
impl_into_err_from_inner!(base64::DecodeError, e => crate::Error { inner: InnerError::Base64Decode(e) });

impl<T> From<std::sync::PoisonError<T>> for crate::Error {

    fn from(_: std::sync::PoisonError<T>) -> crate::Error {
        crate::Error::with("thread poisoned")
    }
}

impl Serialize for crate::Error {

    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match &self.inner {
            InnerError::Raw(msg) => serializer.serialize_str(&msg),
            e => serializer.serialize_str(&e.to_string())
        }
    }
}