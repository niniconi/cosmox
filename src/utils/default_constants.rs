pub const fn default_page_size() -> u64 {
  40
}

/// return ascii letters, number and separators.
///
/// ```rust
/// "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789 -_"
/// ```
pub const fn ascii_letters_number_separators() -> &'static str {
  "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789 -_"
}
