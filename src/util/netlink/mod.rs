pub mod acpi;
pub mod ffi;

use neli::consts::socket::NlFamily;
use neli::err::RouterError;
use neli::genl::Genlmsghdr;
use neli::nl::Nlmsghdr;
use neli::router::asynchronous::NlRouter;
use neli::utils::Groups;
use tokio::sync::mpsc::{self, Receiver};

use self::ffi::{acpi_genl_event, AcpiAttrType, AcpiGenericNetlinkEvent};
use crate::error::{Error, Result};

pub async fn netlink_acpi_listen() -> Result<Receiver<AcpiGenericNetlinkEvent>> {
    // open netlink socket
    let (socket, mut multicast) = NlRouter::connect(NlFamily::Generic, None, Groups::empty())
        .await
        .map_err(|e| -> Error { format!("failed to open socket: {}", e).into() })?;

    // fetch acpi ids
    let family_id = acpi::event_family_id().await?;
    let multicast_group_id = acpi::multicast_group_id().await?;

    // subscribe to multicast events for acpi
    socket.add_mcast_membership(Groups::new_groups(&[multicast_group_id]))?;

    // spawn task to listen and respond to acpi events
    let (tx, rx) = mpsc::channel(8);
    tokio::task::spawn_local(async move {
        // rust-analyzer has trouble figuring this type out, so we help it here a little
        type Payload = Genlmsghdr<u8, u16>;
        type Next = Option<std::result::Result<Nlmsghdr<u16, Payload>, RouterError<u16, Payload>>>;

        loop {
            match multicast.next::<u16, Payload>().await as Next {
                None => break,
                Some(response) => match response {
                    Err(e) => log::error!("error receiving netlink msg: {}", e),
                    Ok(nl_msg) => {
                        // skip this message if it's not part of the apci family
                        if *nl_msg.nl_type() != family_id {
                            continue;
                        }

                        // if it is, then decode it
                        if let Some(payload) = nl_msg.get_payload() {
                            let attrs = payload.attrs().get_attr_handle();
                            if let Some(attr) = attrs.get_attribute(AcpiAttrType::Event as u16) {
                                // cast the attribute payload into its type
                                let raw = attr.nla_payload().as_ref().as_ptr();
                                let event = unsafe { &*(raw as *const acpi_genl_event) };

                                // if there was an error, stop listening and exit
                                match event.try_into() {
                                    Ok(event) => {
                                        if let Err(e) = tx.send(event).await {
                                            log::error!("failed to send acpi event: {}", e);
                                            break;
                                        };
                                    }
                                    Err(e) => log::error!("failed to parse event data: {}", e),
                                }
                            }
                        }
                    }
                },
            }
        }

        // move the socket into here so it's not dropped earlier than expected
        drop(socket);
        log::error!("unexpected end of netlink stream")
    });

    Ok(rx)
}
