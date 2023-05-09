use crate::theme::Theme;

pub fn fraction(theme: &Theme, num: usize, den: usize) -> String {
    if den <= 1 {
        return "".into();
    }

    format!(
        r#" <span foreground="{}"><sup>{}</sup>/<sub>{}</sub></span>"#,
        theme.dark4, num, den
    )
}
