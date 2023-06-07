use std::error::Error;

use neli::consts::socket::NlFamily;
use neli::router::asynchronous::NlRouter;
use neli::utils::Groups;
use tokio::sync::OnceCell;

use super::ffi::{ACPI_EVENT_FAMILY_NAME, ACPI_EVENT_MCAST_GROUP_NAME};

// (family id, multicast group id)
static ACPI_EVENT_IDS: OnceCell<(u16, u32)> = OnceCell::const_new();

async fn init_ids() -> Result<&'static (u16, u32), Box<dyn Error>> {
    Ok(ACPI_EVENT_IDS
        .get_or_try_init(|| get_acpi_id_from_netlink())
        .await?)
}

pub async fn event_family_id() -> Result<u16, Box<dyn Error>> {
    let (family_id, _) = init_ids().await?;
    Ok(*family_id)
}

pub async fn multicast_group_id() -> Result<u32, Box<dyn Error>> {
    let (_, multicast_group_id) = init_ids().await?;
    Ok(*multicast_group_id)
}

async fn get_acpi_id_from_netlink() -> Result<(u16, u32), Box<dyn Error>> {
    // open netlink socket
    let (socket, _) = NlRouter::connect(NlFamily::Generic, None, Groups::empty())
        .await
        .map_err(|e| -> Box<dyn Error> { format!("failed to open socket: {}", e).into() })?;

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
