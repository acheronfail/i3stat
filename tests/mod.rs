macro_rules! assert_json_contains {
    ($haystack:expr, $needle:expr$(,)?) => {
        crate::util::json_contains_inner(&$haystack, &$needle)
    };
}

mod i3;
mod spawn;
mod util;
