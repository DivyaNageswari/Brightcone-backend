use sea_orm::DbErr;

#[derive(Debug)]
pub enum ModelError {
    EntityNotFound,
    EntityAlreadyExists,
    Validation(String),
    Database(String),
    Other(String),
    Any(Box<dyn std::error::Error + Send + Sync>),
}

pub type ModelResult<T> = std::result::Result<T, ModelError>;

impl std::fmt::Display for ModelError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EntityNotFound => write!(f, "Entity not found"),
            Self::EntityAlreadyExists => write!(f, "Entity already exists"),
            Self::Validation(msg) => write!(f, "Validation error: {msg}"),
            Self::Database(msg) => write!(f, "Database error: {msg}"),
            Self::Other(msg) => write!(f, "Error: {msg}"),
            Self::Any(err) => write!(f, "Error: {err}"),
        }
    }
}

impl std::error::Error for ModelError {}

impl From<DbErr> for ModelError {
    fn from(error: DbErr) -> Self {
        match &error {
            DbErr::RecordNotFound(_) => Self::EntityNotFound,
            DbErr::Custom(msg) if msg.contains("unique") || msg.contains("duplicate key") => {
                Self::EntityAlreadyExists
            }
            _ => Self::Database(error.to_string()),
        }
    }
}

impl From<loco_rs::Error> for ModelError {
    fn from(error: loco_rs::Error) -> Self {
        Self::Other(error.to_string())
    }
}

impl From<ModelError> for loco_rs::Error {
    fn from(error: ModelError) -> Self {
        match error {
            ModelError::EntityNotFound => Self::NotFound,
            ModelError::EntityAlreadyExists => Self::BadRequest("Record already exists.".into()),
            ModelError::Validation(message) => Self::BadRequest(message),
            ModelError::Database(_) | ModelError::Other(_) | ModelError::Any(_) => {
                Self::InternalServerError
            }
        }
    }
}
