use num_traits::Float;
use serde_derive::{Deserialize, Serialize};

use crate::theme::Theme;

pub fn fraction(theme: &Theme, num: usize, den: usize) -> String {
    if den <= 1 {
        return "".into();
    }

    // NOTE: need the `line_height` hack so it doesn't change the vertical alignment of other text
    // inside this block
    format!(
        r#" <span line_height="1024" foreground="{}"><sup>{}</sup>/<sub>{}</sub></span>"#,
        theme.dim, num, den
    )
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FloatFormat {
    pad: Option<char>,
    pad_count: Option<usize>,
    #[serde(default)]
    precision: usize,
}

fn num_digits<F: Float>(n: F) -> usize {
    // SAFETY: the input type is constrained to a float, and all f32's fit into an f64
    let n = n.abs().to_f64().unwrap();
    if n < 1.0 {
        1
    } else {
        (n.log10() + 1.).floor() as usize
    }
}

pub fn float<F: Float>(n: F, fmt: &FloatFormat) -> String {
    // SAFETY: the input type is constrained to a float, and all f32's fit into an f64
    let n = n.to_f64().unwrap();
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
