use crate::model::MissingField;

pub type ServerResult<T> = std::result::Result<T, ServerError>;

#[derive(thiserror::Error, Debug)]
pub enum ServerError {
    #[error("Database error: {0}")]
    DatabaseError(#[from] redb::DatabaseError),
    #[error("Table error: {0}")]
    TableError(#[from] redb::TableError),
    #[error("Transaction error: {0}")]
    TransactionError(Box<redb::TransactionError>),
    #[error("IO error: {0}")]
    IOError(#[from] std::io::Error),
    #[error("Storage error: {0}")]
    StorageError(#[from] redb::StorageError),
    #[error("Commit error: {0}")]
    CommitError(#[from] redb::CommitError),
    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),
    #[error("Missing template fields: {0:?}")]
    MissingTemplateFields(Vec<MissingField>),
}

impl From<redb::TransactionError> for ServerError {
    fn from(e: redb::TransactionError) -> Self {
        ServerError::TransactionError(Box::new(e))
    }
}
