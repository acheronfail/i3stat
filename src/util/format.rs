use num_traits::Float;
use serde_derive::{Deserialize, Serialize};

use crate::theme::Theme;

/// Display a fraction (e.g., 1/2) with pango formatting.
pub fn fraction(theme: &Theme, num: usize, den: usize) -> String {
    if den <= 1 {
        return String::new();
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

    let pad_char = fmt.pad.unwrap_or(' ');
    let precision = fmt.precision.unwrap_or(0);
    let pad_count = fmt.pad_count.unwrap_or({
        if precision > 0 {
            // three digits (e.g., 100%) + decimal separator
            3 + 1
        } else {
            // three digits (e.g., 100%) only
            3
        }
    });

    let len = num_digits(n);
    if len >= pad_count {
        format!("{:.precision$}", n, precision = precision)
    } else {
        format!(
            "{}{:.precision$}",
            pad_char.to_string().repeat(pad_count - len),
            n,
            precision = precision
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_num_digits() {
        assert_eq!(num_digits(0.0), 1);
        assert_eq!(num_digits(1.0), 1);
        assert_eq!(num_digits(5.0), 1);
        assert_eq!(num_digits(10.0), 2);
        assert_eq!(num_digits(42.0), 2);
        assert_eq!(num_digits(1729.0), 4);
        assert_eq!(num_digits(1_234_567_890.0), 10);

        assert_eq!(num_digits(-0.0), 1);
        assert_eq!(num_digits(-1.0), 1);
        assert_eq!(num_digits(-5.0), 1);
        assert_eq!(num_digits(-10.0), 2);
        assert_eq!(num_digits(-42.0), 2);
        assert_eq!(num_digits(-1729.0), 4);
        assert_eq!(num_digits(-1_234_567_890.0), 10);
    }

    #[test]
    fn format_default() {
        let fmt = FloatFormat::default();
        assert_eq!(float(0.1, &fmt), "  0");
        assert_eq!(float(1.2, &fmt), "  1");
        assert_eq!(float(10.3, &fmt), " 10");
        assert_eq!(float(100.4, &fmt), "100");
    }

    #[test]
    fn format_just_pad() {
        let fmt = FloatFormat {
            pad: Some('x'),
            ..Default::default()
        };

        assert_eq!(float(0.1, &fmt), "xx0");
        assert_eq!(float(1.2, &fmt), "xx1");
        assert_eq!(float(10.3, &fmt), "x10");
        assert_eq!(float(100.4, &fmt), "100");
    }

    #[test]
    fn format_just_pad_count() {
        let fmt = FloatFormat {
            pad_count: Some(4),
            ..Default::default()
        };

        assert_eq!(float(0.1, &fmt), "   0");
        assert_eq!(float(1.2, &fmt), "   1");
        assert_eq!(float(10.3, &fmt), "  10");
        assert_eq!(float(100.4, &fmt), " 100");
    }

    #[test]
    fn format_just_precision() {
        let fmt = FloatFormat {
            precision: Some(3),
            ..Default::default()
        };

        assert_eq!(float(0.1, &fmt), "   0.100");
        assert_eq!(float(1.2, &fmt), "   1.200");
        assert_eq!(float(10.3, &fmt), "  10.300");
        assert_eq!(float(100.4, &fmt), " 100.400");
    }

    #[test]
    fn format_precision() {
        let fmt = FloatFormat {
            precision: Some(3),
            pad_count: Some(0),
            ..Default::default()
        };

        assert_eq!(float(0.1, &fmt), "0.100");
        assert_eq!(float(1.2, &fmt), "1.200");
        assert_eq!(float(10.3, &fmt), "10.300");
        assert_eq!(float(100.4, &fmt), "100.400");
    }

    #[test]
    fn format_all() {
        let fmt = FloatFormat {
            precision: Some(3),
            pad_count: Some(5),
            pad: Some('-'),
        };

        assert_eq!(float(0.1, &fmt), "----0.100");
        assert_eq!(float(1.2, &fmt), "----1.200");
        assert_eq!(float(10.3, &fmt), "---10.300");
        assert_eq!(float(100.4, &fmt), "--100.400");
        assert_eq!(float(99999.999, &fmt), "99999.999");
    }
}
