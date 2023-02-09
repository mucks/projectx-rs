pub type Result<T> = std::result::Result<T, Error>;

pub enum Error {
    CustomError(String),
}

impl From<String> for Error {
    fn from(err: String) -> Self {
        Self::CustomError(err)
    }
}
