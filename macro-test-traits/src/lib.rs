pub trait Load: for<'de> serde::Deserialize<'de> {
    fn load() -> Result<Self, LoadError>;
}

pub trait LoadStatic: for<'de> serde::Deserialize<'de> {
    fn load_static() -> Self;
}

#[derive(Debug)]
pub enum LoadError {
    FileNotFound,
    ParseError,
    ReadError,
}