pub mod ffi;

use neli::consts::socket::NlFamily;
use neli::err::RouterError;
use neli::genl::Genlmsghdr;
use neli::nl::Nlmsghdr;
use neli::router::asynchronous::NlRouter;
use neli::utils::Groups;
use tokio::sync::mpsc::{self, Receiver};
use tokio::sync::OnceCell;

use self::ffi::{
    acpi_genl_event,
    AcpiAttrType,
    AcpiGenericNetlinkEvent,
    ACPI_EVENT_FAMILY_NAME,
    ACPI_EVENT_MCAST_GROUP_NAME,
};
use crate::error::{Error, Result};

// public ----------------------------------------------------------------------

pub async fn netlink_acpi_listen() -> Result<Receiver<AcpiGenericNetlinkEvent>> {
    // open netlink socket
    let (socket, mut multicast) = NlRouter::connect(NlFamily::Generic, None, Groups::empty())
        .await
        .map_err(|e| -> Error { format!("failed to open socket: {}", e).into() })?;

    // fetch acpi ids
    let family_id = event_family_id().await?;
    let multicast_group_id = multicast_group_id().await?;

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

// internal --------------------------------------------------------------------

// (family id, multicast group id)
static ACPI_EVENT_IDS: OnceCell<(u16, u32)> = OnceCell::const_new();

/// Initialises local cache of the required ACPI netlink ids
/// You can see these with the tool `genl-ctrl-list`.
async fn init_ids() -> Result<&'static (u16, u32)> {
    Ok(ACPI_EVENT_IDS
        .get_or_try_init(get_acpi_id_from_netlink)
        .await?)
}

async fn event_family_id() -> Result<u16> {
    let (family_id, _) = init_ids().await?;
    Ok(*family_id)
}

async fn multicast_group_id() -> Result<u32> {
    let (_, multicast_group_id) = init_ids().await?;
    Ok(*multicast_group_id)
}

/// Use `netlink(3)` to get the right ACPI ids
async fn get_acpi_id_from_netlink() -> Result<(u16, u32)> {
    // open netlink socket
    let (socket, _) = NlRouter::connect(NlFamily::Generic, None, Groups::empty())
        .await
        .map_err(|e| -> Error { format!("failed to open socket: {}", e).into() })?;

    // thanks `neli` - there was so much to do!
    let family_id = socket.resolve_genl_family(ACPI_EVENT_FAMILY_NAME).await?;
    let multicast_group = socket
        .resolve_nl_mcast_group(ACPI_EVENT_FAMILY_NAME, ACPI_EVENT_MCAST_GROUP_NAME)
        .await?;

    Ok((family_id, multicast_group))
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::util::local_block_on;

    #[test]
    fn it_works() {
        local_block_on(async {
            assert!(event_family_id().await.unwrap() > 0);
        })
        .unwrap();

        local_block_on(async {
            assert!(multicast_group_id().await.unwrap() > 0);
        })
        .unwrap();
    }
}
