use serde_derive::{Deserialize, Serialize};

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

#[derive(Debug, Serialize, Deserialize)]
pub struct FloatFormat {
    pad: Option<char>,
    pad_count: Option<usize>,
    #[serde(default)]
    precision: usize,
}

fn num_digits(n: f32) -> usize {
    let n = n.abs();
    if n < 1.0 {
        1
    } else {
        (n.log10() + 1.).floor() as usize
    }
}

pub fn float(n: f32, fmt: &FloatFormat) -> String {
    let pad_count = fmt.pad_count.unwrap_or_else(|| {
        if fmt.precision > 0 {
            // three digits (e.g., 100%) + decimal separator + precision
            3 + 1 + fmt.precision
        } else {
            // three digits (e.g., 100%) only
            3
        }
    });

    let padding = fmt
        .pad
        .map(|c| {
            let s = c.to_string();
            let len = num_digits(n);
            if len >= pad_count {
                "".into()
            } else {
                s.repeat(pad_count - len)
            }
        })
        .unwrap_or("".into());

    format!(
        "{}{:.precision$}",
        padding,
        n,
        precision = fmt.precision as usize,
    )
}
