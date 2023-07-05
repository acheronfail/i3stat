//! Use generic netlink nl80211 in order to gather wireless information such as the SSID, BSSID and signal strength.
//!
//! The following resources were helpful when writing this:
//!     - `/usr/include/linux/nl80211.h`
//!     - https://github.com/jbaublitz/neli/blob/86a0c7a8fdd6db3b19d4971ab58f0d445ca327b5/examples/nl80211.rs#L1
//!     - https://github.com/bmegli/wifi-scan/blob/master/wifi_scan.c
//!     - https://github.com/uoaerg/wavemon
//!     - https://github.com/HewlettPackard/wireless-tools/blob/master/wireless_tools/iwlib.c
//!     - https://github.com/Alamot/code-snippets/blob/master/nl80211_info/nl80211_info.c
//!     - http://lists.infradead.org/pipermail/hostap/2004-March/006231.html
//!     - https://blog.onethinglab.com/how-to-check-if-wireless-adapter-supports-monitor-mode/
//!     - https://git.sipsolutions.net/iw.git/
//!     - https://wireless.wiki.kernel.org/en/users/Documentation/iw

mod enums;

use std::rc::Rc;

use neli::consts::nl::{GenlId, NlmF};
use neli::consts::socket::NlFamily;
use neli::err::RouterError;
use neli::genl::{AttrTypeBuilder, Genlmsghdr, GenlmsghdrBuilder, NlattrBuilder, NoUserHeader};
use neli::nl::{NlPayload, Nlmsghdr};
use neli::router::asynchronous::{NlRouter, NlRouterReceiverHandle};
use neli::types::{Buffer, GenlBuffer};
use neli::utils::Groups;
use tokio::sync::OnceCell;

use self::enums::{
    Nl80211Attribute,
    Nl80211Bss,
    Nl80211Command,
    Nl80211IfType,
    Nl80211StationInfo,
};
use super::NetlinkInterface;
use crate::util::{MacAddr, Result as Res};

// init ------------------------------------------------------------------------

type Nl80211Socket = (NlRouter, NlRouterReceiverHandle<u16, Genlmsghdr<u8, u16>>);
static NL80211_SOCKET: OnceCell<Nl80211Socket> = OnceCell::const_new();
static NL80211_FAMILY: OnceCell<u16> = OnceCell::const_new();

async fn init_socket() -> Res<Nl80211Socket> {
    Ok(NlRouter::connect(NlFamily::Generic, Some(0), Groups::empty()).await?)
}

async fn init_family(socket: &NlRouter) -> Res<u16> {
    Ok(socket.resolve_genl_family("nl80211").await?)
}

// impl ------------------------------------------------------------------------

#[derive(Debug)]
pub struct WirelessInfo {
    /// Wireless interface index
    pub index: i32,
    /// Wireless interface name
    pub interface: Rc<str>,
    /// MAC address of the wireless interface
    pub mac_addr: MacAddr,
    /// SSID of the network; only set when connected to a wireless network
    pub ssid: Option<Rc<str>>,
    /// BSSID of the network; only set when connected to a wireless network
    pub bssid: Option<MacAddr>,
    /// Signal strength of the connection; only set when connected to a wireless network
    pub signal: Option<SignalStrength>,
}

type Payload = Genlmsghdr<Nl80211Command, Nl80211Attribute>;
type NextNl80211 = Option<Result<Nlmsghdr<GenlId, Payload>, RouterError<GenlId, Payload>>>;

impl NetlinkInterface {
    pub async fn wireless_info(&self) -> Option<WirelessInfo> {
        match self.get_wireless_info().await {
            Ok(info) => Some(info),
            Err(e) => {
                log::warn!("NetlinkInterface::wireless_info(): {}", e);
                None
            }
        }
    }

    pub async fn get_wireless_info(&self) -> Res<WirelessInfo> {
        let (socket, _) = NL80211_SOCKET.get_or_try_init(init_socket).await?;
        let family_id = NL80211_FAMILY
            .get_or_try_init(|| init_family(socket))
            .await?;

        // prepare generic message attributes
        let mut attrs = GenlBuffer::new();

        // ... the `Nl80211Command::GetScan` command requires the interface index as `Nl80211Attribute::Ifindex`
        attrs.push(
            NlattrBuilder::default()
                .nla_type(
                    AttrTypeBuilder::default()
                        .nla_type(Nl80211Attribute::Ifindex)
                        .build()?,
                )
                .nla_payload(self.index)
                .build()?,
        );

        let mut recv = socket
            .send::<_, _, u16, Genlmsghdr<Nl80211Command, Nl80211Attribute>>(
                *family_id,
                NlmF::ACK | NlmF::REQUEST,
                NlPayload::Payload(
                    GenlmsghdrBuilder::<Nl80211Command, Nl80211Attribute, NoUserHeader>::default()
                        .cmd(Nl80211Command::GetInterface)
                        .version(1)
                        .attrs(attrs)
                        .build()?,
                ),
            )
            .await?;

        while let Some(Ok(msg)) = recv.next().await as NextNl80211 {
            if let NlPayload::Payload(gen_msg) = msg.nl_payload() {
                let attr_handle = gen_msg.attrs().get_attr_handle();

                // only inspect Station interface types - other types may not be wireless devices
                // this seems to work for my wireless cards, other `Nl80211IfType`'s may need to be
                // added to fully support everything else
                if !matches!(
                    attr_handle.get_attr_payload_as::<Nl80211IfType>(Nl80211Attribute::Iftype),
                    Ok(Nl80211IfType::Station)
                ) {
                    continue;
                }

                // interface name - not really needed since we'll use the index
                let interface = match attr_handle
                    .get_attr_payload_as_with_len::<String>(Nl80211Attribute::Ifname)
                {
                    Ok(name) => name.into(),
                    Err(e) => {
                        log::error!("failed to parse ifname from nl80211 msg: {}", e);
                        "".into()
                    }
                };

                // interface MAC address
                let mac_addr = match attr_handle
                    .get_attr_payload_as_with_len_borrowed::<&[u8]>(Nl80211Attribute::Mac)
                {
                    Ok(bytes) => <&[u8] as TryInto<MacAddr>>::try_into(bytes)?,
                    Err(e) => {
                        log::error!("failed to parse mac from nl80211 msg: {}", e);
                        continue;
                    }
                };

                // NOTE: it seems that nl80211 netlink doesn't null terminate the SSID here, so fetch
                // it as bytes and convert it to a string ourselves
                let ssid = match attr_handle
                    .get_attr_payload_as_with_len_borrowed::<&[u8]>(Nl80211Attribute::Ssid)
                {
                    Ok(name) => Some(String::from_utf8_lossy(name).into()),
                    // if there's no SSID, then the interface is likely not connected to a network
                    Err(_) => None,
                };

                // don't bother fetching these if we don't have an ssid, since the interface is probably
                // not connected to a network
                let (bssid, signal) = {
                    match ssid {
                        Some(_) => {
                            let bssid = get_bssid(socket, self.index).await?;
                            let signal = match bssid.as_ref() {
                                Some(bssid) => {
                                    get_signal_strength(socket, self.index, bssid).await?
                                }
                                None => None,
                            };
                            (bssid, signal)
                        }
                        None => (None, None),
                    }
                };

                return Ok(WirelessInfo {
                    index: self.index,
                    interface,
                    mac_addr,
                    ssid,
                    bssid,
                    signal,
                });
            }
        }

        bail!("no wireless info found for index: {}", self.index);
    }
}

#[derive(Debug, Clone)]
pub struct SignalStrength {
    /// Signal strength in decibels
    pub dbm: i8,
    /// I'm not really sure what this is, but it matches whatever `link` is in `/proc/net/wireless`
    // TODO: find out what it actually is
    pub link: u8,
    /// Best guess of a percentage of network quality
    pub quality: f32,
}

/// Get the current BSSID of the connected network (if any) for the given interface
async fn get_bssid(socket: &NlRouter, index: i32) -> Res<Option<MacAddr>> {
    let family_id = NL80211_FAMILY
        .get_or_try_init(|| init_family(socket))
        .await?;

    // prepare generic message attributes
    let mut attrs = GenlBuffer::new();

    // ... the `Nl80211Command::GetScan` command requires the interface index as `Nl80211Attribute::Ifindex`
    attrs.push(
        NlattrBuilder::default()
            .nla_type(
                AttrTypeBuilder::default()
                    .nla_type(Nl80211Attribute::Ifindex)
                    .build()?,
            )
            .nla_payload(index)
            .build()?,
    );

    // create generic message
    let genl_payload: Genlmsghdr<Nl80211Command, Nl80211Attribute> = GenlmsghdrBuilder::default()
        .cmd(Nl80211Command::GetScan)
        .version(1)
        .attrs(attrs)
        .build()?;

    // send it to netlink
    let mut recv = socket
        .send::<_, _, u16, Genlmsghdr<Nl80211Command, Nl80211Attribute>>(
            *family_id,
            NlmF::DUMP,
            NlPayload::Payload(genl_payload),
        )
        .await?;

    // look for our requested data inside netlink's results
    while let Some(result) = recv.next().await as NextNl80211 {
        match result {
            Ok(msg) => {
                if let NlPayload::Payload(gen_msg) = msg.nl_payload() {
                    let mut attr_handle = gen_msg.attrs().get_attr_handle();

                    if let Ok(bss_attrs) =
                        attr_handle.get_nested_attributes::<Nl80211Bss>(Nl80211Attribute::Bss)
                    {
                        if let Ok(bytes) = bss_attrs
                            .get_attr_payload_as_with_len_borrowed::<&[u8]>(Nl80211Bss::Bssid)
                        {
                            if let Ok(bssid) = MacAddr::try_from(bytes) {
                                return Ok(Some(bssid));
                            }
                        }
                    }
                }
            }
            Err(e) => {
                log::error!("Nl80211Command::GetStation error: {}", e);
                continue;
            }
        }
    }

    Ok(None)
}

/// Gets the signal strength of a wireless network connection
async fn get_signal_strength(
    socket: &NlRouter,
    index: i32,
    bssid: &MacAddr,
) -> Res<Option<SignalStrength>> {
    let family_id = NL80211_FAMILY
        .get_or_try_init(|| init_family(socket))
        .await?;

    // prepare generic message attributes...
    let mut attrs = GenlBuffer::new();

    // ... the `Nl80211Command::GetStation` command requires the interface index as `Nl80211Attribute::Ifindex`...
    attrs.push(
        NlattrBuilder::default()
            .nla_type(
                AttrTypeBuilder::default()
                    .nla_type(Nl80211Attribute::Ifindex)
                    .build()?,
            )
            .nla_payload(index)
            .build()?,
    );

    // ... and also the BSSID as `Nl80211Attribute::Mac`
    attrs.push(
        NlattrBuilder::default()
            .nla_type(
                AttrTypeBuilder::default()
                    .nla_type(Nl80211Attribute::Mac)
                    .build()?,
            )
            .nla_payload(Buffer::from(bssid))
            .build()?,
    );

    // create generic message
    let genl_payload: Genlmsghdr<Nl80211Command, Nl80211Attribute> = GenlmsghdrBuilder::default()
        .cmd(Nl80211Command::GetStation)
        .version(1)
        .attrs(attrs)
        .build()?;

    // send it to netlink
    let mut recv = socket
        .send::<_, _, u16, Genlmsghdr<Nl80211Command, Nl80211Attribute>>(
            *family_id,
            NlmF::ACK | NlmF::REQUEST,
            NlPayload::Payload(genl_payload),
        )
        .await?;

    // look for our requested data inside netlink's results
    while let Some(result) = recv.next().await as NextNl80211 {
        match result {
            Ok(msg) => {
                if let NlPayload::Payload(gen_msg) = msg.nl_payload() {
                    let mut attr_handle = gen_msg.attrs().get_attr_handle();

                    // FIXME: upstream - I don't think this needs to be mutable...
                    if let Ok(station_info) = attr_handle
                        .get_nested_attributes::<Nl80211StationInfo>(Nl80211Attribute::StaInfo)
                    {
                        if let Ok(signal) =
                            station_info.get_attr_payload_as::<u8>(Nl80211StationInfo::Signal)
                        {
                            // this is the same as `/proc/net/wireless`'s `link`
                            let link = 110_u8.wrapping_add(signal);
                            // this is the same as `/proc/net/wireless`'s `level`
                            let dbm = signal as i8;
                            // just a guess at a percentage - there's not really a good way to represent this easily
                            //  - https://github.com/bmegli/wifi-scan/issues/18
                            //  - https://github.com/psibi/iwlib-rs/blob/master/src/lib.rs#L48
                            //  - https://www.intuitibits.com/2016/03/23/dbm-to-percent-conversion/
                            //  - https://eyesaas.com/wi-fi-signal-strength/
                            let quality = (if dbm < -110 {
                                0_f32
                            } else if dbm > -40 {
                                100_f32
                            } else {
                                (dbm + 40).abs() as f32 / 70.0
                            }) * 100.0;

                            return Ok(Some(SignalStrength { dbm, link, quality }));
                        }
                    }
                }
            }
            Err(e) => {
                log::error!("Nl80211Command::GetStation error: {}", e);
                continue;
            }
        }
    }

    Ok(None)
}
