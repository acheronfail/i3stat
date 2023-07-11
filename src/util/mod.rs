use_and_export!(cell, enum_cycle, exec, format, net, netlink, paginator, path, vec);

use futures::Future;
use tokio::runtime::{Builder, Runtime};
use tokio::task::LocalSet;

use crate::error::Result;

/// Block on a given future, running it on the current thread inside a `LocalSet`.
pub fn local_block_on<F>(f: F) -> Result<(F::Output, Runtime)>
where
    F: Future,
{
    let runtime = Builder::new_current_thread().enable_all().build()?;
    // NOTE: this `LocalSet` must be wrapped in an `async` block, otherwise a panic occurs when the
    // `LocalSet` begins dropping all of its tasks.
    // See: https://github.com/tokio-rs/tokio/discussions/5794
    // And: https://github.com/dbus2/zbus/issues/380
    let output = runtime.block_on(async { LocalSet::new().run_until(f).await });
    Ok((output, runtime))
}
