use std::os::unix::prelude::OsStrExt;
use std::path::{Path, PathBuf};

use wordexp::{wordexp, Wordexp};

use crate::error::Result;

pub fn expand_path(path: impl AsRef<Path>) -> Result<PathBuf> {
    // SAFETY: there's no need to do conversions ot UTF-8 checks here, since `wordexp` immediately
    // converts the `&str` to a `CString` to pass it to C code. So, just re-interpret the given path
    // as a `&str` and pass it on
    let s = unsafe { std::str::from_utf8_unchecked(path.as_ref().as_os_str().as_bytes()) };

    let mut expand = wordexp(s, Wordexp::new(0), 0)?;
    // only take the first
    match expand.next() {
        Some(first) => Ok(PathBuf::from(first)),
        // is this even reachable?
        None => bail!("expansion resulted in nothing"),
    }
}

#[cfg(test)]
mod tests {
    use std::ffi::OsStr;

    use super::*;

    #[test]
    fn it_works() {
        assert_eq!(
            expand_path(PathBuf::from("path")).unwrap(),
            PathBuf::from("path")
        );

        assert_eq!(
            expand_path(PathBuf::from("~/path")).unwrap(),
            PathBuf::from(format!("{}/path", std::env::var("HOME").unwrap()))
        );
    }

    #[test]
    #[should_panic(expected = "expansion resulted in nothing")]
    fn passthrough_to_wordexp() {
        let invalid_utf8 = vec![1, 159, 146, 150];
        let os_str = OsStr::from_bytes(&invalid_utf8);
        expand_path(PathBuf::from(os_str)).unwrap();
    }
}
