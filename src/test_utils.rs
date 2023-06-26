use std::env;
use std::error::Error;
use std::fs::{create_dir_all, write};
use std::path::{Path, PathBuf};

use clap::Command;
use clap_mangen::Man;

// man pages -------------------------------------------------------------------

pub fn generate_manpage(cmd: Command) {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("doc");
    m(&cmd, &dir, None).expect("failed to generate manpage");
}

fn m(
    cmd: &Command,
    dir: &Path,
    parent_name: Option<&str>,
) -> std::result::Result<(), Box<dyn Error>> {
    let man = Man::new(cmd.clone());
    let mut buf = Vec::new();
    man.render(&mut buf)?;

    let cmd_name = cmd.get_display_name().unwrap_or_else(|| cmd.get_name());
    let file_name = match parent_name {
        Some(parent_name) => format!("{}-{}.1", parent_name, cmd_name),
        None => format!("{}.1", cmd_name),
    };

    create_dir_all(&dir)?;
    write(dir.join(&file_name), buf)?;

    for sub in cmd.get_subcommands() {
        m(sub, dir, Some(&cmd_name))?;
    }

    Ok(())
}

/// Simply checks if we're running the tests via the justfile (by checking the existence of an env var).
/// This is because we spawn the tests in a nested dbus session - so any interactions with dbus don't
/// affect the host environment.
#[test]
fn ensure_proper_test_env() {
    assert!(
        std::env::var("ISTAT_TEST").is_ok(),
        "Please ensure you're running the tests via the justfile!"
    );
}
