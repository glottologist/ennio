use std::borrow::Cow;

pub fn escape(s: &str) -> Cow<'_, str> {
    shell_escape::escape(Cow::Borrowed(s))
}
