mod github;
mod linear;

pub use github::GitHubTracker;
pub use linear::LinearTracker;

pub(crate) fn truncate_to_char_boundary(s: &str, max_bytes: usize) -> usize {
    if max_bytes >= s.len() {
        return s.len();
    }
    let mut end = max_bytes;
    while end > 0 && !s.is_char_boundary(end) {
        end -= 1;
    }
    end
}

pub(crate) fn sanitize_title_for_branch(title: &str, max_bytes: usize) -> String {
    let sanitized: String = title
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' {
                c.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect();

    let trimmed = sanitized.trim_matches('-');
    let end = truncate_to_char_boundary(trimmed, max_bytes);
    let truncated = &trimmed[..end];
    truncated.trim_end_matches('-').to_owned()
}
