mod generated;

use std::{
    error::Error,
    sync::{
        atomic::{
            AtomicBool,
            Ordering,
        },
        Arc,
    },
    thread::JoinHandle,
    time::Duration,
};

use dbus::{
    arg::Variant,
    blocking::Connection,
    channel::MatchingReceiver,
    message::MatchRule,
    Message,
};
use generated::OrgDunstprojectCmd0;

use super::{
    Item,
    ToItem,
};

pub struct Dunst {
    paused: Arc<AtomicBool>,
}

impl Dunst {
    fn get_initial_paused() -> Result<bool, Box<dyn Error>> {
        let c = Connection::new_session().unwrap();
        let p = c.with_proxy(
            "org.freedesktop.Notifications",
            "/org/freedesktop/Notifications",
            Duration::from_secs(5),
        );

        let paused = p.paused()?;
        Ok(paused)
    }

    fn start_monitor(item: &Dunst) -> Result<JoinHandle<()>, Box<dyn Error>> {
        let b = item.paused.clone();

        // TODO: `unwrap()`s
        let handle = std::thread::spawn(move || {
            let c = Connection::new_session().unwrap();
            let p = c.with_proxy(
                "org.freedesktop.DBus",
                "/org/freedesktop/DBus",
                Duration::from_secs(5),
            );

            let m = MatchRule::new()
                .with_type(dbus::MessageType::MethodCall)
                .with_path("/org/freedesktop/Notifications")
                .with_interface("org.freedesktop.DBus.Properties")
                .with_member("Set");

            // https://github.com/diwic/dbus-rs/blob/master/dbus/examples/monitor.rs
            let _: () = p
                .method_call(
                    "org.freedesktop.DBus.Monitoring",
                    "BecomeMonitor",
                    (vec![m.match_str()], 0u32),
                )
                .unwrap();

            c.start_receive(
                m,
                Box::new(move |msg: Message, _| {
                    let (_, what, is_paused): (&str, &str, Variant<bool>) = msg.read3().unwrap();
                    if what == "paused" {
                        b.store(is_paused.0, Ordering::SeqCst);
                    }

                    // return true to continue monitoring
                    true
                }),
            );

            loop {
                c.process(Duration::from_millis(1000)).unwrap();
            }
        });

        Ok(handle)
    }
}

impl Default for Dunst {
    fn default() -> Self {
        let paused = Dunst::get_initial_paused().unwrap();
        let paused = Arc::new(AtomicBool::new(paused));
        let item = Dunst { paused };
        Dunst::start_monitor(&item).unwrap();
        item
    }
}

impl ToItem for Dunst {
    fn to_item(&self) -> Item {
        if self.paused.load(Ordering::SeqCst) {
            Item::text(" DnD ")
        } else {
            Item::text("")
        }
    }
}
