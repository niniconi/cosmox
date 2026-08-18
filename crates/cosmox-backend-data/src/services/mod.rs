pub mod auth;
pub mod device_service;
pub mod file_service;
pub mod init_service;
pub mod jwt;
pub mod libraries_service;
pub mod metadata_service;
pub mod resource_service;
pub mod role_permission_service;
pub mod scanner_service;
pub mod search_service;
pub mod system_service;
pub mod tag_service;
pub mod user_service;

/// Truncate a description to at most `max_chars` chars.
///
/// Cut on char boundaries (not bytes) so multi-byte UTF-8 is never split:
/// the `description` DB columns are `VARCHAR(512)`, which limits by chars.
pub fn truncate_description(description: Option<String>, max_chars: usize) -> Option<String> {
    description.map(|description| {
        if description.chars().count() <= max_chars {
            description
        } else {
            description.chars().take(max_chars).collect()
        }
    })
}

pub(crate) const DESCRIPTION_MAX_CHARS: usize = 512;

#[cfg(test)]
mod tests {
    use super::{DESCRIPTION_MAX_CHARS, truncate_description};

    #[test]
    fn truncate_keeps_short_description() {
        let desc = Some("short".to_string());
        assert_eq!(truncate_description(desc, 512), Some("short".to_string()));
    }

    #[test]
    fn truncate_cuts_long_description_on_char_boundary() {
        let desc = Some("中文字符串".repeat(200));
        let truncated = truncate_description(desc, DESCRIPTION_MAX_CHARS).unwrap();
        assert_eq!(truncated.chars().count(), DESCRIPTION_MAX_CHARS);
        assert!(truncated.is_char_boundary(truncated.len()));
    }

    #[test]
    fn truncate_preserves_none() {
        assert_eq!(truncate_description(None, 512), None);
    }

    #[test]
    fn truncate_exact_limit_is_kept() {
        let desc = Some("a".repeat(DESCRIPTION_MAX_CHARS));
        assert_eq!(
            truncate_description(desc, DESCRIPTION_MAX_CHARS)
                .unwrap()
                .chars()
                .count(),
            DESCRIPTION_MAX_CHARS
        );
    }
}
