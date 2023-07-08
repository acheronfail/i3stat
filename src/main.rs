use std::process;

use clap::Parser;
use hex_color::HexColor;
use istat::cli::Cli;
use istat::config::AppConfig;
use istat::context::{Context, SharedState, StopAction};
use istat::dispatcher::Dispatcher;
use istat::error::Result;
use istat::i3::header::I3BarHeader;
use istat::i3::ipc::handle_click_events;
use istat::i3::{I3Item, I3Markup};
use istat::ipc::{create_ipc_socket, handle_ipc_events, IpcContext};
use istat::signals::handle_signals;
use istat::theme::Theme;
use istat::util::{local_block_on, RcCell};
use tokio::sync::mpsc::{self, Receiver};
use tokio::time::Instant;
use tokio_util::sync::CancellationToken;

enum RuntimeStopReason {
    Shutdown,
}

fn main() {
    match start_runtime() {
        Ok(RuntimeStopReason::Shutdown) => {}
        Err(err) => {
            log::error!("{}", err);
            process::exit(1);
        }
    }
}

fn start_runtime() -> Result<RuntimeStopReason> {
    pretty_env_logger::try_init_timed()?;

    let args = Cli::parse();

    let (result, runtime) = local_block_on(async_main(args))?;

    // NOTE: since we use tokio's stdin implementation which spawns a background thread and blocks,
    // we have to shutdown the runtime ourselves here. If we didn't, then when the runtime is
    // dropped it would block indefinitely until that background thread unblocked (i.e., another
    // JSON line from i3).
    // Thus, if anything other than the stdin task fails, we have to manually shut it down here.
    // See: https://github.com/tokio-rs/tokio/discussions/5684
    runtime.shutdown_background();

    result
}

async fn async_main(args: Cli) -> Result<RuntimeStopReason> {
    let config = RcCell::new(AppConfig::read(args).await?);

    // create socket first, so it's ready before anything is written to stdout
    let socket = create_ipc_socket(&config).await?;

    // create i3 bar and spawn tasks for each bar item
    let (bar, dispatcher) = setup_i3_bar(&config)?;

    // handle incoming signals
    let signal_handle = handle_signals(config.clone(), dispatcher.clone())?;

    // used to handle app shutdown
    let token = CancellationToken::new();

    // ipc context
    let ipc_ctx = IpcContext::new(
        bar.clone(),
        token.clone(),
        config.clone(),
        dispatcher.clone(),
    );

    // handle our inputs: i3's IPC and our own IPC
    let result = tokio::select! {
        Err(err) = handle_ipc_events(socket, ipc_ctx) => Err(err),
        Err(err) = handle_click_events(dispatcher.clone()) => Err(err),
        _ = token.cancelled() => Ok(RuntimeStopReason::Shutdown),
    };

    // if we reach here, then something went wrong, so clean up
    signal_handle.close();
    return result;
}

fn setup_i3_bar(config: &RcCell<AppConfig>) -> Result<(RcCell<Vec<I3Item>>, RcCell<Dispatcher>)> {
    let item_count = config.items.len();

    // shared state
    let state = SharedState::new();

    // A list of items which represents the i3 bar
    let bar = RcCell::new(vec![I3Item::empty(); item_count]);

    // Used to send events to each bar item, and also to trigger updates of the bar
    let (update_tx, update_rx) = mpsc::channel(1);
    let dispatcher = RcCell::new(Dispatcher::new(update_tx, item_count));

    // Used by items to send updates back to the bar
    let (item_tx, item_rx) = mpsc::channel(item_count + 1);

    // Iterate config and create bar items
    for (idx, item) in config.items.iter().enumerate() {
        let bar_item = item.to_bar_item();

        // all cheaply cloneable (smart pointers, senders, etc)
        let mut bar = bar.clone();
        let state = state.clone();
        let config = config.clone();
        let item_tx = item_tx.clone();
        let mut dispatcher = dispatcher.clone();

        tokio::task::spawn_local(async move {
            let mut retries = 0;
            let mut last_start;
            loop {
                last_start = Instant::now();
                let (event_tx, event_rx) = mpsc::channel(32);
                dispatcher.set(idx, event_tx);

                let ctx = Context::new(
                    config.clone(),
                    state.clone(),
                    item_tx.clone(),
                    event_rx,
                    idx,
                );

                let fut = bar_item.start(ctx);
                match fut.await {
                    Ok(StopAction::Restart) => {
                        // reset retries if no retries have occurred in the last 5 minutes
                        if last_start.elapsed().as_secs() > 60 * 5 {
                            retries = 0;
                        }

                        // restart if we haven't exceeded limit
                        if retries < 3 {
                            log::warn!("item[{}] requested restart...", idx);
                            retries += 1;
                            continue;
                        }

                        // we exceeded the limit, so error out
                        log::error!("item[{}] stopped, exceeded max retries", idx);
                        let theme = config.theme.clone();
                        bar[idx] = I3Item::new("MAX RETRIES")
                            .color(theme.bg)
                            .background_color(theme.red);

                        break;
                    }
                    // since this item has terminated, remove its entry from the bar
                    action @ Ok(StopAction::Complete) | action @ Ok(StopAction::Remove) => {
                        log::info!("item[{}] finished running", idx);
                        dispatcher.remove(idx);

                        // Remove this item if requested
                        if matches!(action, Ok(StopAction::Remove)) {
                            // NOTE: wait for all tasks in queue so any remaining item updates are flushed and processed
                            // before we set it for the last time here
                            tokio::task::yield_now().await;
                            // replace with an empty item
                            bar[idx] = I3Item::empty();
                        }

                        break;
                    }
                    // unexpected error, log and display an error block
                    Err(e) => {
                        log::error!("item[{}] exited with error: {}", idx, e);
                        // replace with an error item
                        let theme = config.theme.clone();
                        bar[idx] = I3Item::new("ERROR")
                            .color(theme.bg)
                            .background_color(theme.red);
                        break;
                    }
                }
            }
        });
    }

    // setup listener for handling item updates and printing the bar to STDOUT
    handle_item_updates(config.clone(), item_rx, update_rx, bar.clone())?;

    Ok((bar, dispatcher))
}

// task to manage updating the bar and printing it as JSON
fn handle_item_updates(
    config: RcCell<AppConfig>,
    mut item_rx: Receiver<(I3Item, usize)>,
    mut update_rx: Receiver<()>,
    mut bar: RcCell<Vec<I3Item>>,
) -> Result<()> {
    // output first parts of the i3 bar protocol - the header
    println!("{}", serde_json::to_string(&I3BarHeader::default())?);
    // and the opening bracket for the "infinite array"
    println!("[");

    tokio::task::spawn_local(async move {
        let item_names = config.item_idx_to_name();

        loop {
            tokio::select! {
                // a manual update was requested
                Some(()) = update_rx.recv() => {}
                // an item is requesting an update, update the bar state
                Some((i3_item, idx)) = item_rx.recv() => {
                    let mut i3_item = i3_item
                        // the name of the item
                        .name(item_names[idx].clone())
                        // always override the bar item's `instance`, since we track that ourselves
                        .instance(idx.to_string());

                    if let Some(separator) = config.items[idx].common.separator {
                        i3_item = i3_item.separator(separator);
                    }

                    // don't bother doing anything if the item hasn't changed
                    if bar[idx] == i3_item {
                        continue;
                    }

                    // update item in bar
                    bar[idx] = i3_item;
                }
            }

            // serialise to JSON
            let theme = config.theme.clone();
            let bar_json = match theme.powerline_enable {
                true => serde_json::to_string(&create_powerline(
                    &bar,
                    &theme,
                    &make_color_adjuster(&theme.bg, &theme.dim),
                )),
                false => serde_json::to_string(&*bar),
            };

            // print bar to STDOUT for i3
            match bar_json {
                // make sure to include the trailing comma `,` as part of the protocol
                Ok(json) => println!("{},", json),
                // on any serialisation error, emit an error that will be drawn to the status bar
                Err(e) => {
                    log::error!("failed to serialise bar to json: {}", e);
                    println!(
                        r#"[{{"full_text":"FATAL ERROR: see logs in stderr","color":"black","background":"red"}}],"#
                    );
                }
            }
        }
    });

    Ok(())
}

fn create_powerline<F>(bar: &[I3Item], theme: &Theme, adjuster: F) -> Vec<I3Item>
where
    F: Fn(&HexColor) -> HexColor,
{
    let len = theme.powerline.len();
    let mut powerline_bar = vec![];
    let mut powerline_idx = 0;
    for i in 0..bar.len() {
        let item = &bar[i];
        if item.full_text.is_empty() {
            continue;
        }

        let instance = i.to_string();
        #[cfg(debug_assertions)]
        assert_eq!(item.get_instance().unwrap(), &instance);

        let c1 = &theme.powerline[powerline_idx % len];
        let c2 = &theme.powerline[(powerline_idx + 1) % len];
        powerline_idx += 1;

        // create the powerline separator
        let mut sep_item = I3Item::new(theme.powerline_separator.to_span())
            .instance(instance)
            .separator(false)
            .markup(I3Markup::Pango)
            .separator_block_width_px(0)
            .color(c2.bg)
            .with_data("powerline_sep", true.into());

        // the first separator doesn't blend with any other item
        if i > 0 {
            sep_item = sep_item.background_color(c1.bg);
        }

        // replace `config.theme.dim` so it's easy to see
        let adjusted_dim = adjuster(&c2.bg);

        powerline_bar.push(sep_item);
        powerline_bar.push(
            item.clone()
                .full_text(format!(
                    " {} ",
                    // replace `config.theme.dim` use in pango spans
                    item.full_text
                        .replace(&theme.dim.to_string(), &adjusted_dim.to_string())
                ))
                .separator(false)
                .separator_block_width_px(0)
                .color(match item.get_color() {
                    Some(color) if color == &theme.dim => adjusted_dim,
                    Some(color) => *color,
                    _ => c2.fg,
                })
                .background_color(c2.bg),
        );
    }
    powerline_bar
}

/// HACK: this assumes that RGB colours scale linearly - I don't know if they do or not.
/// Used to render the powerline bar and make sure that dim text is visible.
fn make_color_adjuster(bg: &HexColor, fg: &HexColor) -> impl Fn(&HexColor) -> HexColor {
    let r = fg.r.abs_diff(bg.r);
    let g = fg.g.abs_diff(bg.g);
    let b = fg.b.abs_diff(bg.b);
    move |c| {
        HexColor::rgb(
            r.saturating_add(c.r),
            g.saturating_add(c.g),
            b.saturating_add(c.b),
        )
    }
}
