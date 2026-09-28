use thiserror::Error;

/// Unique indexes (see migrations) whose violation means a live sibling
/// already uses the requested name.
const NAME_CONSTRAINTS: [&str; 2] = [
    "unique_live_name_per_parent",
    "unique_live_root_name_per_owner",
];

#[derive(Debug, Error)]
pub enum MetadataError {
    #[error("node not found")]
    NotFound,
    #[error("a node with this name already exists in the destination folder")]
    NameConflict,
    #[error("invalid name: {0}")]
    InvalidName(&'static str),
    #[error("invalid file content: {0}")]
    InvalidContent(&'static str),
    #[error("node is not a folder")]
    NotAFolder,
    #[error("node is not a file")]
    NotAFile,
    #[error("cannot move a folder into itself or one of its descendants")]
    CycleDetected,
    #[error("node is not in the trash")]
    NotTrashed,
    #[error("the parent folder is in the trash; restore it first")]
    ParentTrashed,
    #[error("database error: {0}")]
    Database(sqlx::Error),
}

impl From<sqlx::Error> for MetadataError {
    fn from(err: sqlx::Error) -> Self {
        if let sqlx::Error::Database(db_err) = &err
            && db_err
                .constraint()
                .is_some_and(|constraint| NAME_CONSTRAINTS.contains(&constraint))
        {
            return Self::NameConflict;
        }
        Self::Database(err)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn not_found_has_a_stable_message() {
        assert_eq!(MetadataError::NotFound.to_string(), "node not found");
    }

    #[test]
    fn invalid_name_includes_the_reason() {
        let err = MetadataError::InvalidName("name must not contain '/'");
        assert_eq!(err.to_string(), "invalid name: name must not contain '/'");
    }

    #[test]
    fn non_constraint_database_errors_stay_database_errors() {
        let err: MetadataError = sqlx::Error::RowNotFound.into();
        assert!(matches!(err, MetadataError::Database(_)));
    }
}
