use num_traits::Float;
use serde_derive::{Deserialize, Serialize};

use crate::theme::Theme;

/// Display a fraction (e.g., 1/2) with pango formatting.
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

/// Common, re-usable options for formatting floats.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FloatFormat {
    /// The character to use for padding.
    pad: Option<char>,
    /// How many characters to pad with. If unset, will pad to 3 digits before the
    /// decimal point - useful for percentages.
    pad_count: Option<usize>,
    /// The precision displayed when formatting the float.
    precision: Option<usize>,
}

/// Return the number of digits (before the decimal place) a given number has.
fn num_digits<F: Float>(n: F) -> usize {
    // SAFETY: the input type is constrained to a float, and all f32's fit into an f64
    let n = n.abs().to_f64().unwrap();
    if n < 1.0 {
        1
    } else {
        (n.log10() + 1.).floor() as usize
    }
}

/// Format a float according to the given options.
pub fn float<F: Float>(n: F, fmt: &FloatFormat) -> String {
    // SAFETY: the input type is constrained to a float, and all f32's fit into an f64
    let n = n.to_f64().unwrap();
    if matches!((fmt.pad, fmt.pad_count, fmt.precision), (None, None, None)) {
        return format!("{:3.0}", n);
    }

    let precision = fmt.precision.unwrap_or(0);
    let pad_count = fmt.pad_count.unwrap_or_else(|| {
        if precision > 0 {
            // three digits (e.g., 100%) + decimal separator + precision
            3 + 1 + precision
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

    format!("{}{:.precision$}", padding, n, precision = precision,)
}
