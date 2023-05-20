use std::error::Error;
use std::ffi::OsStr;
use std::io::{Read, Write};
use std::os::unix::net::UnixStream;
use std::path::PathBuf;

use clap::builder::PossibleValue;
use clap::{ColorChoice, Parser, Subcommand, ValueEnum};
use istat::i3::{I3Button, I3ClickEvent, I3Modifier};
use istat::ipc::{get_socket_path, IpcBarEvent, IpcMessage, IpcReply};

#[derive(Debug, Parser)]
#[clap(color = ColorChoice::Always)]
struct Cli {
    #[command(subcommand)]
    cmd: CliCommand,
    /// Path to the socket to use for ipc
    #[clap(long)]
    socket: Option<PathBuf>,
}

#[derive(Debug, Subcommand)]
enum CliCommand {
    /// Returns information about the currently running bar
    Info,
    /// Send a click event to a bar item
    Click {
        /// The target bar item: can be an index or the name of the item
        target: String,
        /// The mouse button to send
        button: Button,
        /// A list of modifiers (pass multiple times) emulated in the click event
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

fn send_message(
    socket_path: impl AsRef<OsStr>,
    msg: IpcMessage,
) -> Result<IpcReply, Box<dyn Error>> {
    let mut stream = UnixStream::connect(socket_path.as_ref())?;
    if let Err(e) = stream.write_all(&serde_json::to_vec(&msg)?) {
        return Err(format!("Error writing to socket: {}", e).into());
    }

    let mut buf = vec![];
    let n = match stream.read_to_end(&mut buf) {
        Ok(n) => n,
        Err(e) => {
            return Err(format!("Error reading from socket: {}", e).into());
        }
    };

    Ok(serde_json::from_slice(&buf[..n])?)
}

fn main() -> Result<(), Box<dyn Error>> {
    let args = Cli::parse();
    let socket_path = get_socket_path(args.socket)?;

    let msg = match args.cmd {
        CliCommand::Info => IpcMessage::Info,
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
            IpcMessage::BarEvent {
                instance: target,
                event,
            }
        }
        CliCommand::Signal { target } => IpcMessage::BarEvent {
            instance: target,
            event: IpcBarEvent::Signal,
        },
        CliCommand::Custom { target, args } => IpcMessage::BarEvent {
            instance: target,
            event: IpcBarEvent::Custom(args),
        },
    };

    let resp = send_message(&socket_path, msg)?;
    println!(
        "{}",
        match resp {
            IpcReply::Help(help) => help,
            IpcReply::Response(value) => value.to_string(),
            x => serde_json::to_string(&x)?,
        }
    );

    Ok(())
}
