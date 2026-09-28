use std::fmt::Write;

/// Characters RFC 5987 allows unescaped in an `ext-value` besides ASCII
/// letters and digits.
const ATTR_CHAR_PUNCTUATION: &[u8] = b"!#$&+-.^_`|~";

/// Builds an RFC 6266 `attachment` Content-Disposition value that is safe for
/// any file name: a sanitized ASCII `filename` for old clients plus the exact
/// name, percent-encoded, as `filename*`.
pub(crate) fn attachment_disposition(file_name: &str) -> String {
    let fallback: String = file_name
        .chars()
        .map(|c| {
            if c == ' ' || (c.is_ascii_graphic() && c != '"' && c != '\\') {
                c
            } else {
                '_'
            }
        })
        .collect();

    let mut encoded = String::with_capacity(file_name.len());
    for byte in file_name.bytes() {
        if byte.is_ascii_alphanumeric() || ATTR_CHAR_PUNCTUATION.contains(&byte) {
            encoded.push(char::from(byte));
        } else {
            write!(encoded, "%{byte:02X}").expect("writing to a String cannot fail");
        }
    }

    format!("attachment; filename=\"{fallback}\"; filename*=UTF-8''{encoded}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plain_ascii_names_are_kept_as_is() {
        assert_eq!(
            attachment_disposition("report.pdf"),
            "attachment; filename=\"report.pdf\"; filename*=UTF-8''report.pdf"
        );
    }

    #[test]
    fn non_ascii_names_get_an_ascii_fallback_and_an_exact_utf8_form() {
        assert_eq!(
            attachment_disposition("café ☕.txt"),
            "attachment; filename=\"caf_ _.txt\"; filename*=UTF-8''caf%C3%A9%20%E2%98%95.txt"
        );
    }

    #[test]
    fn hostile_characters_cannot_break_the_header() {
        let value = attachment_disposition("a\"b\\c\r\nSet-Cookie: x.txt");

        assert_eq!(
            value,
            "attachment; filename=\"a_b_c__Set-Cookie: x.txt\"; \
             filename*=UTF-8''a%22b%5Cc%0D%0ASet-Cookie%3A%20x.txt"
        );
        assert!(!value.contains('\r') && !value.contains('\n'));
    }
}
