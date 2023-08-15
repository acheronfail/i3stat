use std::ffi::OsStr;
use std::io::{Read, Write};
use std::os::unix::net::UnixStream;
use std::path::PathBuf;

use clap::builder::PossibleValue;
use clap::{ColorChoice, Parser, Subcommand, ValueEnum};
use istat::bail;
use istat::error::Result;
use istat::i3::{I3Button, I3ClickEvent, I3Modifier};
use istat::ipc::get_socket_path;
use istat::ipc::protocol::{encode_ipc_msg, IpcBarEvent, IpcMessage, IpcReply, IPC_HEADER_LEN};
use serde_json::Value;

#[derive(Debug, Parser)]
#[clap(name = "istat-ipc", color = ColorChoice::Always)]
struct Cli {
    #[command(subcommand)]
    cmd: CliCommand,
    /// Path to the socket to use for ipc.
    #[clap(long)]
    socket: Option<PathBuf>,
}

#[derive(Debug, Subcommand)]
enum CliCommand {
    /// Returns information about the currently running bar.
    Info,
    /// Sends a signal to all events to trigger a refresh. Note that some items completely ignore all
    /// events, and thus won't receive this refresh events.
    RefreshAll,
    /// Returns the current bar as JSON.
    GetBar,
    /// Returns the current configuration.
    GetConfig {
        /// JSON Pointer for the config https://datatracker.ietf.org/doc/html/rfc6901
        /// If not provided, the entire config will be returned.
        pointer: Option<String>,
    },
    /// Returns the current theme.
    GetTheme {
        /// JSON Pointer for the theme https://datatracker.ietf.org/doc/html/rfc6901
        /// If not provided, the entire theme will be returned.
        pointer: Option<String>,
    },
    /// Update the theme at runtime, some examples:
    ///
    /// `istat-ipc set-theme "/powerline_enable" true`
    /// `istat-ipc set-theme "/powerline_separator/value" "$(printf "\xee\x82\xbe")"`
    /// `istat-ipc set-theme "" "{new theme as json...}"`
    SetTheme {
        /// JSON Pointer for the theme https://datatracker.ietf.org/doc/html/rfc6901
        pointer: String,
        /// New value to set
        json_value: String,
    },
    /// Send a click event to a bar item.
    Click {
        /// The target bar item: can be an index or the name of the item.
        target: String,
        /// The mouse button to send.
        button: Button,
        /// A list of modifiers (pass multiple times) emulated in the click event.
        #[clap(long, short)]
        modifiers: Vec<Modifier>,
        #[clap(long, short)]
        x: Option<usize>,
        #[clap(long, short)]
        y: Option<usize>,
        #[clap(long)]
        relative_x: Option<usize>,
        #[clap(long)]
        relative_y: Option<usize>,
        #[clap(long)]
        output_x: Option<usize>,
        #[clap(long)]
        output_y: Option<usize>,
        #[clap(long, short = 'W')]
        width: Option<usize>,
        #[clap(long, short = 'H')]
        height: Option<usize>,
    },
    /// Send a signal event to a bar item, this is the same as setting `signal=1` in the config file
    /// and then sending the signal (e.g., `pkill -RTMIN+1 istat`)
    Signal {
        /// The target bar item: can be an index or the name of the item
        target: String,
    },
    /// Send a custom event to a bar item. Only a few bar items support custom events, see the documentation for details
    Custom {
        /// The target bar item: can be an index or the name of the item
        target: String,
        /// Arguments to send to the bar item
        #[clap(trailing_var_arg = true)]
        args: Vec<String>,
    },
    Shutdown,
}

#[derive(Debug, Clone)]
struct Button(I3Button);

impl ValueEnum for Button {
    fn value_variants<'a>() -> &'a [Self] {
        &[
            Self(I3Button::Left),
            Self(I3Button::Middle),
            Self(I3Button::Right),
            Self(I3Button::ScrollUp),
            Self(I3Button::ScrollDown),
        ]
    }

    fn to_possible_value(&self) -> Option<PossibleValue> {
        match self.0 {
            I3Button::Left => Some(PossibleValue::new("left")),
            I3Button::Middle => Some(PossibleValue::new("middle")),
            I3Button::Right => Some(PossibleValue::new("right")),
            I3Button::ScrollUp => Some(PossibleValue::new("scroll_up")),
            I3Button::ScrollDown => Some(PossibleValue::new("scroll_down")),
            I3Button::ScrollRight => Some(PossibleValue::new("scroll_right")),
            I3Button::ScrollLeft => Some(PossibleValue::new("scroll_left")),
            _ => None,
        }
    }
}

#[derive(Debug, Clone)]
struct Modifier(I3Modifier);

impl ValueEnum for Modifier {
    fn value_variants<'a>() -> &'a [Self] {
        &[
            Self(I3Modifier::Control),
            Self(I3Modifier::Mod1),
            Self(I3Modifier::Mod2),
            Self(I3Modifier::Mod3),
            Self(I3Modifier::Mod4),
            Self(I3Modifier::Mod5),
            Self(I3Modifier::Shift),
        ]
    }

    fn to_possible_value(&self) -> Option<PossibleValue> {
        match self.0 {
            I3Modifier::Control => Some(PossibleValue::new("control")),
            I3Modifier::Mod1 => Some(PossibleValue::new("mod1")),
            I3Modifier::Mod2 => Some(PossibleValue::new("mod2")),
            I3Modifier::Mod3 => Some(PossibleValue::new("mod3")),
            I3Modifier::Mod4 => Some(PossibleValue::new("mod4")),
            I3Modifier::Mod5 => Some(PossibleValue::new("mod5")),
            I3Modifier::Shift => Some(PossibleValue::new("shift")),
        }
    }
}

fn send_message(socket_path: impl AsRef<OsStr>, msg: IpcMessage) -> Result<IpcReply> {
    let mut stream = UnixStream::connect(socket_path.as_ref())?;

    let msg = encode_ipc_msg(msg)?;
    if let Err(e) = stream.write_all(&msg) {
        bail!("Error writing to socket: {}", e);
    }

    let mut buf = vec![];
    let n = match stream.read_to_end(&mut buf) {
        Ok(n) => n,
        Err(e) => {
            bail!("Error reading from socket: {}", e);
        }
    };

    Ok(serde_json::from_slice(&buf[IPC_HEADER_LEN..n])?)
}

fn send_and_print_response(socket_path: impl AsRef<OsStr>, msg: IpcMessage) -> Result<()> {
    let resp = match send_message(&socket_path, msg) {
        Ok(resp) => resp,
        Err(e) => bail!("failed to send ipc message: {}", e),
    };

    println!(
        "{}",
        match resp {
            IpcReply::Help(help) => help,
            IpcReply::Value(value) => value.to_string(),
            x => serde_json::to_string(&x)?,
        }
    );

    Ok(())
}

fn get_json_response(socket_path: &PathBuf, msg: IpcMessage) -> Result<Value> {
    Ok(match send_message(socket_path, msg)? {
        IpcReply::Value(json) => json,
        _ => unreachable!(),
    })
}

fn main() -> Result<()> {
    let args = Cli::parse();
    let socket_path = get_socket_path(args.socket.as_ref())?;

    match args.cmd {
        CliCommand::Shutdown => send_and_print_response(&socket_path, IpcMessage::Shutdown)?,
        CliCommand::Info => send_and_print_response(&socket_path, IpcMessage::Info)?,
        CliCommand::GetBar => send_and_print_response(&socket_path, IpcMessage::GetBar)?,
        CliCommand::RefreshAll => send_and_print_response(&socket_path, IpcMessage::RefreshAll)?,
        CliCommand::GetConfig { pointer: None } => {
            send_and_print_response(&socket_path, IpcMessage::GetConfig)?
        }
        CliCommand::GetTheme { pointer: None } => {
            send_and_print_response(&socket_path, IpcMessage::GetTheme)?
        }
        CliCommand::GetConfig {
            pointer: Some(pointer),
        } => {
            let config = get_json_response(&socket_path, IpcMessage::GetConfig)?;
            match config.pointer(&pointer) {
                Some(value) => println!("{}", value),
                None => bail!("No value found at: {}", pointer),
            }
        }
        CliCommand::GetTheme {
            pointer: Some(pointer),
        } => {
            let theme = get_json_response(&socket_path, IpcMessage::GetTheme)?;
            match theme.pointer(&pointer) {
                Some(value) => println!("{}", value),
                None => bail!("No value found at: {}", pointer),
            }
        }
        CliCommand::SetTheme {
            pointer,
            json_value,
        } => {
            let mut theme = get_json_response(&socket_path, IpcMessage::GetTheme)?;
            match theme.pointer_mut(&pointer) {
                Some(value) => {
                    let trimmed = json_value.trim();
                    let new_value = match serde_json::from_str::<Value>(&trimmed) {
                        // passed a direct JSON value
                        Ok(value) => Ok(value),
                        // assume string if it doesn't definitely look like some JSON value
                        Err(_) if !trimmed.starts_with(&['[', '{', '\'', '"']) => {
                            Ok(Value::String(trimmed.into()))
                        }
                        // pass through any other error
                        err => err,
                    }?;

                    // update the current config - note that this may not be correct, for example if
                    // the user passed a string where a boolean was expected
                    *value = new_value;

                    // send config back via IPC
                    send_and_print_response(&socket_path, IpcMessage::SetTheme(theme))?;
                }
                None => bail!("No value found at: {}", pointer),
            }
        }
        CliCommand::Click {
            target,
            button,
            modifiers,
            x,
            y,
            relative_x,
            relative_y,
            output_x,
            output_y,
            width,
            height,
        } => {
            let mut click = I3ClickEvent::default();
            click.button = button.0;
            click.instance = Some(target.clone());
            click.modifiers = modifiers.into_iter().map(|m| m.0).collect();
            x.map(|x| click.x = x);
            y.map(|y| click.y = y);
            relative_x.map(|relative_x| click.relative_x = relative_x);
            relative_y.map(|relative_y| click.relative_y = relative_y);
            output_x.map(|output_x| click.output_x = output_x);
            output_y.map(|output_y| click.output_y = output_y);
            width.map(|width| click.width = width);
            height.map(|height| click.height = height);

            let event = IpcBarEvent::Click(click);
            send_and_print_response(
                &socket_path,
                IpcMessage::BarEvent {
                    instance: target,
                    event,
                },
            )?;
        }
        CliCommand::Signal { target } => send_and_print_response(
            &socket_path,
            IpcMessage::BarEvent {
                instance: target,
                event: IpcBarEvent::Signal,
            },
        )?,
        CliCommand::Custom { target, args } => send_and_print_response(
            &socket_path,
            IpcMessage::BarEvent {
                instance: target,
                event: IpcBarEvent::Custom(args),
            },
        )?,
    }

    Ok(())
}

#[cfg(test)]
#[path = "../src/test_utils.rs"]
mod test_utils;

#[cfg(test)]
mod tests {
    use clap::CommandFactory;

    use super::test_utils::generate_manpage;
    use super::*;

    #[test]
    fn manpage() {
        generate_manpage(Cli::command());
    }
}
