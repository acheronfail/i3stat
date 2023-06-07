use_and_export!(cell, enum_cycle, exec, format, net, netlink, paginator);

use std::error::Error;

use futures::Future;
use tokio::runtime::{Builder, Runtime};
use tokio::task::LocalSet;

/// Block on a given future, running it on the current thread inside a `LocalSet`.
pub fn local_block_on<F>(f: F) -> Result<(F::Output, Runtime), Box<dyn Error>>
where
    F: Future,
{
    let runtime = Builder::new_current_thread().enable_all().build()?;
    let output = LocalSet::new().block_on(&runtime, f);
    Ok((output, runtime))
}
