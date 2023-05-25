use std::cell::RefCell;
use std::collections::HashMap;
use std::convert::Infallible;
use std::error::Error;
use std::process;
use std::rc::Rc;
use std::time::Duration;

use clap::Parser;
use hex_color::HexColor;
use istat::cli::Cli;
use istat::config::AppConfig;
use istat::context::{Context, SharedState};
use istat::dispatcher::Dispatcher;
use istat::i3::header::I3BarHeader;
use istat::i3::ipc::handle_click_events;
use istat::i3::{I3Item, I3Markup};
use istat::ipc::handle_ipc_events;
use istat::signals::handle_signals;
use tokio::sync::mpsc::{self, Receiver};

fn main() {
    if let Err(err) = start_runtime() {
        log::error!("{}", err);
        process::exit(1);
    }
}

fn start_runtime() -> Result<Infallible, Box<dyn Error>> {
    pretty_env_logger::try_init()?;

    let args = Cli::parse();

    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;

    let result = tokio::task::LocalSet::new().block_on(&runtime, async_main(args));

    // NOTE: we use tokio's stdin implementation which spawns a background thread and blocks,
    // we have to shutdown the runtime ourselves here. If we didn't, then when the runtime is
    // dropped it would block indefinitely until that background thread unblocked (i.e., another
    // JSON line from i3).
    // Thus, if anything other than the stdin task fails, we have to manually shut it down here.
    // See: https://github.com/tokio-rs/tokio/discussions/5684
    runtime.shutdown_timeout(Duration::from_secs(1));

    result
}

type Bar = Rc<RefCell<Vec<I3Item>>>;

async fn async_main(args: Cli) -> Result<Infallible, Box<dyn Error>> {
    let config = AppConfig::read(args).await?;

    println!("{}", serde_json::to_string(&I3BarHeader::default())?);
    println!("[");

    let item_count = config.items.len();

    // shared context
    let state = SharedState::new();

    // state for the bar (moved to bar_printer)
    let bar: Bar = Rc::new(RefCell::new(vec![I3Item::empty(); item_count]));
    let mut bar_txs = vec![];

    // for each BarItem, spawn a new task to manage it
    let (item_tx, item_rx) = mpsc::channel(item_count + 1);
    for (idx, item) in config.items.iter().enumerate() {
        let bar_item = item.to_bar_item();

        let (event_tx, event_rx) = mpsc::channel(32);
        bar_txs.push(event_tx.clone());

        let ctx = Context::new(
            config.theme.clone(),
            state.clone(),
            item_tx.clone(),
            event_tx,
            event_rx,
            idx,
        );
        let bar = bar.clone();
        tokio::task::spawn_local(async move {
            let theme = ctx.theme.clone();
            let fut = bar_item.start(ctx);
            // since this item has terminated, remove its entry from the bar
            match fut.await {
                Ok(()) => {
                    log::info!("item[{}] finished running", idx);
                    // replace with an empty item
                    bar.borrow_mut()[idx] = I3Item::empty();
                }
                Err(e) => {
                    log::error!("item[{}] exited with error: {}", idx, e);
                    // replace with an error item
                    bar.borrow_mut()[idx] = I3Item::new("ERROR")
                        .color(theme.bg)
                        .background_color(theme.red);
                }
            }
        });
    }

    let dispatcher = Dispatcher::new(
        bar_txs
            .into_iter()
            .enumerate()
            .map(|(idx, tx)| (idx, (tx, config.items[idx].clone())))
            .collect::<HashMap<usize, _>>(),
    );

    // setup listener for handling item updates and printing the bar to STDOUT
    handle_item_updates(config.clone(), item_rx, bar);

    // handle incoming signals
    let signal_handle = handle_signals(config.socket(), dispatcher.clone())?;

    // handle our inputs: i3's IPC and our own IPC
    let err = tokio::select! {
        err = handle_ipc_events(&config, dispatcher.clone()) => err,
        err = handle_click_events(dispatcher.clone()) => err,
    };

    // if we reach here, then something went wrong while reading STDIN, so clean up
    signal_handle.close();
    return err;
}

// task to manage updating the bar and printing it as JSON
fn handle_item_updates(config: AppConfig, mut rx: Receiver<(I3Item, usize)>, bar: Bar) {
    let item_names = config
        .items
        .iter()
        .map(|item| match item.common.name {
            Some(ref name) => name.to_string(),
            None => item.tag().into(),
        })
        .collect::<Vec<_>>();

    tokio::task::spawn_local(async move {
        let adjuster = make_color_adjuster(&config.theme.bg, &config.theme.dim);

        while let Some((i3_item, idx)) = rx.recv().await {
            let mut bar = bar.borrow_mut();
            bar[idx] = i3_item
                // the name of the item
                .name(item_names[idx].clone())
                // always override the bar item's `instance`, since we track that ourselves
                .instance(idx.to_string());

            let bar_json = match config.theme.powerline_enable {
                true => serde_json::to_string(&create_powerline(&*bar, &config, &adjuster)),
                false => serde_json::to_string(&*bar),
            };

            // print bar to STDOUT for i3
            match bar_json {
                Ok(json) => println!("{},", json),
                Err(e) => {
                    log::error!("failed to serialise bar to json: {}", e);
                    println!(
                        r#"[{{"full_text":"FATAL ERROR: see logs in stderr","color":"black","background":"red"}}],"#
                    );
                }
            }
        }
    });
}

fn create_powerline<F>(bar: &[I3Item], config: &AppConfig, adjuster: F) -> Vec<I3Item>
where
    F: Fn(&HexColor) -> HexColor,
{
    let len = config.theme.powerline.len();
    let mut powerline_bar = vec![];
    for i in 0..bar.len() {
        let item = &bar[i];
        if item.full_text.is_empty() {
            continue;
        }

        let instance = i.to_string();
        #[cfg(debug_assertions)]
        assert_eq!(item.get_instance().unwrap(), &instance);

        let c1 = &config.theme.powerline[i % len];
        let c2 = &config.theme.powerline[(i + 1) % len];

        // create the powerline separator
        let mut sep_item = I3Item::new(config.theme.powerline_separator.to_span())
            .instance(instance)
            .separator(false)
            .markup(I3Markup::Pango)
            .separator_block_width_px(0)
            .color(c2.bg);

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
                        .replace(&config.theme.dim.to_string(), &adjusted_dim.to_string())
                ))
                .separator(false)
                .separator_block_width_px(0)
                .color(match item.get_color() {
                    Some(color) if color == &config.theme.dim => adjusted_dim,
                    Some(color) => *color,
                    _ => c2.fg,
                })
                .background_color(c2.bg),
        );
    }
    powerline_bar
}

/// HACK: this assumes that RGB colours scale linearly - I don't know if they do or not.
/// Used in to render the powerline bar and make sure that dim text is visible.
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
