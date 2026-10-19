#[derive(Debug)]
pub enum ModelError {
    EntityNotFound,
    EntityAlreadyExists,
    Validation(String),
    Database(String),
    Other(String),
}

pub type ModelResult<T> = std::result::Result<T, ModelError>;

impl From<diesel::result::Error> for ModelError {
    fn from(error: diesel::result::Error) -> Self {
        match error {
            diesel::result::Error::NotFound => Self::EntityNotFound,
            diesel::result::Error::DatabaseError(
                diesel::result::DatabaseErrorKind::UniqueViolation,
                _,
            ) => Self::EntityAlreadyExists,
            error => Self::Database(error.to_string()),
        }
    }
}

impl From<ModelError> for loco_rs::Error {
    fn from(error: ModelError) -> Self {
        match error {
            ModelError::EntityNotFound => Self::NotFound,
            ModelError::EntityAlreadyExists => Self::BadRequest("Record already exists.".into()),
            ModelError::Validation(message) => Self::BadRequest(message),
            ModelError::Database(_) | ModelError::Other(_) => Self::InternalServerError,
        }
    }
}
