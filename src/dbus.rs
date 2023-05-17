use std::cell::RefCell;
use std::error::Error;
use std::ops::Deref;
use std::rc::Rc;
use std::sync::Arc;
use std::time::Duration;

use dbus::channel::MatchingReceiver;
use dbus::message::MatchRule;
use dbus::nonblock::LocalConnection;
use dbus::{nonblock, Message};
use dbus_tokio::connection;

use crate::dispatcher::Dispatcher;

#[derive(Debug, Copy, Clone)]
pub enum BusType {
    Session,
    System,
}

#[derive(Debug)]
pub struct DbusInterest {
    // TODO: support system bus
    #[allow(unused)]
    bus: BusType,
    rule: MatchRule<'static>,
    item_idx: usize,
}

impl DbusInterest {
    pub fn new(item_idx: usize, bus: BusType, rule: MatchRule<'static>) -> DbusInterest {
        DbusInterest {
            item_idx,
            bus,
            rule,
        }
    }
}

#[derive(Debug)]
pub struct DbusMessage(pub Message);

impl Deref for DbusMessage {
    type Target = Message;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

pub fn dbus_connect() -> Result<Arc<LocalConnection>, Box<dyn Error>> {
    let (resource, con) = connection::new_session_local()?;
    tokio::task::spawn_local(async move {
        let err = resource.await;
        log::error!("unexpected disconnect from dbus: {}", err);
    });

    Ok(con)
}

pub async fn dbus_subscribe(
    dispatcher: Dispatcher,
    interests: Vec<DbusInterest>,
) -> Result<(), Box<dyn Error>> {
    if interests.is_empty() {
        return Ok(());
    }

    // NOTE: once a connection has requested "BecomeMonitor", it can't be used for any other dbus
    // method calls, so we create a new connection here just for subscriptions
    let con = dbus_connect()?;

    // request to become a monitor
    {
        let dbus_proxy = nonblock::Proxy::new(
            "org.freedesktop.DBus",
            "/org/freedesktop/DBus",
            Duration::from_secs(5),
            con.clone(),
        );

        // tell dbus we're going to become a monitor
        // https://dbus.freedesktop.org/doc/dbus-specification.html#bus-messages-become-monitor
        let _: () = dbus_proxy
            .method_call(
                "org.freedesktop.DBus.Monitoring",
                "BecomeMonitor",
                (
                    interests
                        .iter()
                        .map(|i| i.rule.match_str())
                        .collect::<Vec<String>>(),
                    0u32,
                ),
            )
            .await?;
    }

    // prepare all the listeners to dispatch messages to the items
    // TODO: is there an "async" way to stream response from a monitor? (rather than this hack)
    // See: https://github.com/diwic/dbus-rs/issues/431
    let dispatcher = Rc::new(RefCell::new(dispatcher));
    for interest in interests {
        let dispatcher = dispatcher.clone();
        con.start_receive(
            interest.rule.clone(),
            Box::new(move |msg: Message, _con: &LocalConnection| {
                let dispatcher = dispatcher.clone();
                tokio::task::spawn_local(async move {
                    let _ = dispatcher
                        .borrow()
                        .send_bar_event(
                            interest.item_idx,
                            crate::context::BarEvent::DbusMessage(DbusMessage(msg)),
                        )
                        .await;
                });

                true
            }),
        );
    }

    Ok(())
}
