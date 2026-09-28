use crate::{MetadataError, NewContent};

const MAX_NAME_BYTES: usize = 255;

pub(crate) fn validate_name(name: &str) -> Result<(), MetadataError> {
    let reason = if name.trim().is_empty() {
        "name must not be empty"
    } else if name.len() > MAX_NAME_BYTES {
        "name must be at most 255 bytes"
    } else if name == "." || name == ".." {
        "name must not be '.' or '..'"
    } else if name.contains('/') {
        "name must not contain '/'"
    } else if name.chars().any(char::is_control) {
        "name must not contain control characters"
    } else if name.trim() != name {
        "name must not start or end with whitespace"
    } else {
        return Ok(());
    };
    Err(MetadataError::InvalidName(reason))
}

pub(crate) fn validate_mime_type(mime_type: &str) -> Result<(), MetadataError> {
    if mime_type.trim().is_empty() {
        return Err(MetadataError::InvalidContent("mime type must not be empty"));
    }
    Ok(())
}

pub(crate) fn validate_content(content: &NewContent) -> Result<(), MetadataError> {
    if content.storage_key.trim().is_empty() {
        return Err(MetadataError::InvalidContent(
            "storage key must not be empty",
        ));
    }
    if content.size_bytes < 0 {
        return Err(MetadataError::InvalidContent("size must not be negative"));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rejection(name: &str) -> &'static str {
        match validate_name(name) {
            Err(MetadataError::InvalidName(reason)) => reason,
            other => panic!("expected {name:?} to be rejected, got {other:?}"),
        }
    }

    #[test]
    fn accepts_ordinary_names() {
        for name in ["report.pdf", "My Documents", "café ☕", ".hidden", "a b"] {
            assert!(validate_name(name).is_ok(), "{name:?} should be accepted");
        }
        assert!(validate_name(&"a".repeat(255)).is_ok());
    }

    #[test]
    fn rejects_empty_and_whitespace_only_names() {
        assert_eq!(rejection(""), "name must not be empty");
        assert_eq!(rejection("   "), "name must not be empty");
    }

    #[test]
    fn rejects_names_longer_than_255_bytes() {
        assert_eq!(
            rejection(&"a".repeat(256)),
            "name must be at most 255 bytes"
        );
        // 128 two-byte characters = 256 bytes.
        assert_eq!(
            rejection(&"é".repeat(128)),
            "name must be at most 255 bytes"
        );
    }

    #[test]
    fn rejects_dot_names_slashes_and_control_characters() {
        assert_eq!(rejection("."), "name must not be '.' or '..'");
        assert_eq!(rejection(".."), "name must not be '.' or '..'");
        assert_eq!(rejection("a/b"), "name must not contain '/'");
        assert_eq!(
            rejection("a\0b"),
            "name must not contain control characters"
        );
        assert_eq!(
            rejection("a\nb"),
            "name must not contain control characters"
        );
    }

    #[test]
    fn rejects_leading_or_trailing_whitespace() {
        assert_eq!(
            rejection(" report"),
            "name must not start or end with whitespace"
        );
        assert_eq!(
            rejection("report "),
            "name must not start or end with whitespace"
        );
    }

    #[test]
    fn validates_mime_type_and_content() {
        assert!(validate_mime_type("application/pdf").is_ok());
        assert!(matches!(
            validate_mime_type(" "),
            Err(MetadataError::InvalidContent(_))
        ));

        let valid = NewContent {
            storage_key: "objects/abc".to_string(),
            size_bytes: 0,
            checksum_sha256: None,
        };
        assert!(validate_content(&valid).is_ok());

        let negative = NewContent {
            size_bytes: -1,
            ..valid.clone()
        };
        assert!(matches!(
            validate_content(&negative),
            Err(MetadataError::InvalidContent("size must not be negative"))
        ));

        let no_key = NewContent {
            storage_key: String::new(),
            ..valid
        };
        assert!(matches!(
            validate_content(&no_key),
            Err(MetadataError::InvalidContent(
                "storage key must not be empty"
            ))
        ));
    }
}
