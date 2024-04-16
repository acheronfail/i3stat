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
//!
//! Some things for me to remember:
//!     - the `nl_type` in a generic netlink payload is the family id

mod enums;

use std::rc::Rc;
use std::result::Result as StdRes;

use neli::consts::nl::{GenlId, NlmF};
use neli::consts::socket::NlFamily;
use neli::err::RouterError;
use neli::genl::{AttrTypeBuilder, Genlmsghdr, GenlmsghdrBuilder, NlattrBuilder};
use neli::nl::{NlPayload, Nlmsghdr};
use neli::router::asynchronous::{NlRouter, NlRouterReceiverHandle};
use neli::types::{Buffer, GenlBuffer};
use neli::utils::Groups;
use tokio::sync::OnceCell;

use self::enums::{
    Nl80211Attribute, Nl80211Bss, Nl80211Command, Nl80211IfType, Nl80211StationInfo,
};
use super::NetlinkInterface;
use crate::util::nl80211::enums::Nl80211BssStatus;
use crate::util::{MacAddr, Result};

// init ------------------------------------------------------------------------

type Nl80211Socket = (NlRouter, NlRouterReceiverHandle<u16, Genlmsghdr<u8, u16>>);
static NL80211_SOCKET: OnceCell<Nl80211Socket> = OnceCell::const_new();
static NL80211_FAMILY: OnceCell<u16> = OnceCell::const_new();
const NL80211_FAMILY_NAME: &str = "nl80211";

async fn init_socket() -> Result<Nl80211Socket> {
    Ok(NlRouter::connect(NlFamily::Generic, Some(0), Groups::empty()).await?)
}

async fn init_family(socket: &NlRouter) -> Result<u16> {
    let id = socket.resolve_genl_family(NL80211_FAMILY_NAME).await?;
    log::debug!("nl80211 family id: {id}");

    Ok(id)
}

// util ------------------------------------------------------------------------

// Explicitly type these since the compiler struggles to infer `neli` types in async contexts.
type Nl80211Payload = Genlmsghdr<Nl80211Command, Nl80211Attribute>;
type NextNl80211 =
    Option<StdRes<Nlmsghdr<GenlId, Nl80211Payload>, RouterError<GenlId, Nl80211Payload>>>;

/// Easily create a `GenlBuffer` with the given attributes and payloads.
macro_rules! attrs {
    () => {
        GenlBuffer::new()
    };

    ($($attr:ident => $payload:expr$(,)?)+) => {{
        let mut genl_attrs = GenlBuffer::new();
        $(
            genl_attrs.push(
                NlattrBuilder::default()
                    .nla_type(AttrTypeBuilder::default().nla_type(Nl80211Attribute::$attr).build()?)
                    .nla_payload($payload)
                    .build()?
            );
        )+

        genl_attrs
    }};
}

/// Send an nl80211 command via generic netlink, and get its response.
/// Build the `attrs` parameter with the `attrs!()` macro.
async fn genl80211_send(
    socket: &NlRouter,
    cmd: Nl80211Command,
    flags: NlmF,
    attrs: GenlBuffer<Nl80211Attribute, Buffer>,
) -> Result<NlRouterReceiverHandle<u16, Nl80211Payload>> {
    let family_id = *NL80211_FAMILY
        .get_or_try_init(|| init_family(socket))
        .await?;

    log::trace!(
        "genl80211_send: cmd={cmd:?}, flags={flags:?}, attrs={:02x?}",
        attrs
            .iter()
            .map(|attr| (attr.nla_type().nla_type(), attr.nla_payload().as_ref()))
            .collect::<Vec<_>>()
    );

    // create generic netlink message
    let genl_payload: Nl80211Payload = {
        let mut builder = GenlmsghdrBuilder::default().version(1).cmd(cmd);
        if !attrs.is_empty() {
            builder = builder.attrs(attrs);
        }

        builder.build()?
    };

    // send it to netlink

    Ok(socket
        .send::<_, _, u16, Nl80211Payload>(family_id, flags, NlPayload::Payload(genl_payload))
        .await?)
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

impl NetlinkInterface {
    /// Get wireless information for this interface (if there is any).
    pub async fn wireless_info(&self) -> Option<WirelessInfo> {
        match self.get_wireless_info().await {
            Ok(info) => info,
            Err(e) => {
                log::error!(
                    "index {} NetlinkInterface::wireless_info(): {}",
                    self.index,
                    e
                );
                None
            }
        }
    }

    /// Gets wireless information for this interface.
    /// Returns `None` if the interface was not a wireless interface, or if no wireless information
    /// could be found.
    async fn get_wireless_info(&self) -> Result<Option<WirelessInfo>> {
        log::trace!("index {} getting wireless info", self.index);

        let (socket, _) = NL80211_SOCKET.get_or_try_init(init_socket).await?;
        let mut recv = genl80211_send(
            socket,
            Nl80211Command::GetInterface,
            NlmF::ACK | NlmF::REQUEST,
            attrs![Ifindex => self.index],
        )
        .await?;

        while let Some(result) = recv.next().await as NextNl80211 {
            let msg = match result {
                Ok(msg) => msg,
                Err(e) => {
                    log::error!(
                        "index {} error occurred receiving nl80211 message: {}",
                        self.index,
                        e
                    );
                    continue;
                }
            };

            if let NlPayload::Payload(gen_msg) = msg.nl_payload() {
                let attr_handle = gen_msg.attrs().get_attr_handle();

                // only inspect Station interface types - other types may not be wireless devices
                // this seems to work for my wireless cards, other `Nl80211IfType`'s may need to be
                // added to fully support everything else
                if !matches!(
                    attr_handle.get_attr_payload_as::<Nl80211IfType>(Nl80211Attribute::Iftype),
                    Ok(Nl80211IfType::Station)
                ) {
                    log::debug!("index {} interface is not a station", self.index);
                    continue;
                }

                // interface name - not really needed since we'll use the index
                let interface = match attr_handle
                    .get_attr_payload_as_with_len::<String>(Nl80211Attribute::Ifname)
                {
                    Ok(name) => name.into(),
                    Err(e) => {
                        log::error!(
                            "index {} failed to parse ifname from nl80211 msg: {}",
                            self.index,
                            e
                        );
                        "".into()
                    }
                };

                // interface MAC address
                let mac_addr = match attr_handle
                    .get_attr_payload_as_with_len_borrowed::<&[u8]>(Nl80211Attribute::Mac)
                {
                    Ok(bytes) => <&[u8] as TryInto<MacAddr>>::try_into(bytes)?,
                    Err(e) => {
                        log::error!(
                            "index {} failed to parse mac from nl80211 msg: {}",
                            self.index,
                            e
                        );
                        continue;
                    }
                };

                // NOTE: it seems that nl80211 netlink doesn't null terminate the SSID here, so fetch
                // it as bytes and convert it to a string ourselves
                let mut ssid = match attr_handle
                    .get_attr_payload_as_with_len_borrowed::<&[u8]>(Nl80211Attribute::Ssid)
                {
                    Ok(name) => {
                        let ssid = String::from_utf8_lossy(name).into();
                        log::debug!("index {} found ssid: {}", self.index, ssid);

                        Some(ssid)
                    }
                    // sometimes the `GetInterface` response doesn't include the ssid, but that's okay
                    // since we can fetch it later when we send a `GetScan` request
                    // see: https://github.com/systemd/systemd/issues/24585
                    Err(_) => None,
                };

                // NOTE: if we didn't get the ssid before, it's also returned in the `GetScan` response
                // so pass it in here too just in case
                let bssid = get_scan(socket, self.index, &mut ssid).await?;
                let signal = match bssid.as_ref() {
                    Some(bssid) => get_signal_strength(socket, self.index, bssid).await?,
                    // we can't fetch the signal strength without the bssid
                    None => None,
                };

                return Ok(Some(WirelessInfo {
                    index: self.index,
                    interface,
                    mac_addr,
                    ssid,
                    bssid,
                    signal,
                }));
            }
        }

        log::debug!("index {} no wireless information found", self.index);
        Ok(None)
    }
}

#[derive(Debug, Clone)]
pub struct SignalStrength {
    /// Signal strength in decibels
    pub dbm: i8,
    /// I'm not really sure what this is, but it matches whatever `link` is in `/proc/net/wireless`
    // TODO: find out what it actually is
    pub link: u8,

    /// Cached quality value
    quality: std::cell::OnceCell<f32>,
}

impl SignalStrength {
    pub fn new(dbm: i8) -> SignalStrength {
        SignalStrength {
            dbm,
            link: 110_u8.wrapping_add(dbm as u8),
            quality: std::cell::OnceCell::new(),
        }
    }

    /// Just a guess at a percentage - there's not really a good way to represent this easily
    ///  - https://github.com/bmegli/wifi-scan/issues/18
    ///  - https://github.com/psibi/iwlib-rs/blob/master/src/lib.rs#L48
    ///  - https://www.intuitibits.com/2016/03/23/dbm-to-percent-conversion/
    ///  - https://eyesaas.com/wi-fi-signal-strength/
    pub fn quality(&self) -> f32 {
        *self.quality.get_or_init(|| {
            (if self.dbm < -110 {
                0_f32
            } else if self.dbm > -40 {
                1_f32
            } else {
                // lerp between -70 and 0
                1.0 - ((self.dbm as f32 + 40.0) / -70.0)
            }) * 100.0
        })
    }
}

// NOTE: see iw's scan.c for a reference of how to parse information elements
// e.g.: https://git.sipsolutions.net/iw.git/tree/scan.c#n1718
const INFORMATION_ELEMENT_SSID: u8 = 0;
fn ssid_from_ie(information_elements: &[u8]) -> Result<Option<Rc<str>>> {
    let mut idx = 0;
    while idx < information_elements.len() {
        let el_type = information_elements[idx];
        let el_len = information_elements[idx + 1] as usize;
        if el_type == INFORMATION_ELEMENT_SSID {
            return Ok(Some(
                String::from_utf8(information_elements[idx + 2..idx + 2 + el_len].to_vec())?.into(),
            ));
        }

        idx += el_len + 2;
    }

    Ok(None)
}

/// Get the current BSSID of the connected network (if any) for the given interface and also
/// optionally set the ssid if it's not already set previously
async fn get_scan(
    socket: &NlRouter,
    index: i32,
    ssid: &mut Option<Rc<str>>,
) -> Result<Option<MacAddr>> {
    let mut recv = genl80211_send(
        socket,
        Nl80211Command::GetScan,
        NlmF::REQUEST | NlmF::ACK | NlmF::ROOT | NlmF::MATCH,
        attrs![Ifindex => index],
    )
    .await?;

    // look for our requested data inside netlink's results - GetScan returns a list of scanned stations...
    while let Some(result) = recv.next().await as NextNl80211 {
        match result {
            Ok(msg) => {
                if let NlPayload::Payload(gen_msg) = msg.nl_payload() {
                    let attr_handle = gen_msg.attrs().get_attr_handle();

                    // extract the `Nl80211Bss` attributes so we can inspect them
                    if let Ok(bss_attrs) =
                        attr_handle.get_nested_attributes::<Nl80211Bss>(Nl80211Attribute::Bss)
                    {
                        // extract the bssid itself
                        let bssid = match bss_attrs
                            .get_attr_payload_as_with_len_borrowed::<&[u8]>(Nl80211Bss::Bssid)
                            .map(MacAddr::try_from)
                        {
                            Ok(Ok(mac_addr)) => mac_addr,
                            // if we can't find a bssid, then keep looking for another one
                            _ => continue,
                        };

                        // NOTE: this is the important part - we find the bssid that the device is currently associated
                        // with! The GetScan can return multiple stations (even when we're connected to one) but we only
                        // want the information about the station we're currently associated with
                        if let Ok(Nl80211BssStatus::Associated) =
                            bss_attrs.get_attr_payload_as::<Nl80211BssStatus>(Nl80211Bss::Status)
                        {
                            log::debug!("index {} found associated bssid: {}", index, bssid);

                            // set the ssid if it's not set already, this works around a quirk in netlink
                            // see: https://github.com/systemd/systemd/issues/24585
                            // either way, this seems to be the more "reliable" way of getting the ssid
                            if ssid.is_none() {
                                // parse the information elements attributes
                                if let Ok(bytes) = bss_attrs
                                    .get_attr_payload_as_with_len_borrowed::<&[u8]>(
                                        Nl80211Bss::InformationElements,
                                    )
                                {
                                    match ssid_from_ie(bytes) {
                                        Ok(option) => {
                                            *ssid = option;
                                            log::debug!(
                                            "index {} updated ssid {:?} from information elements",
                                            index,
                                            ssid
                                        );
                                        }
                                        Err(e) => log::error!(
                                            "index {} failed to parse information elements: {}",
                                            index,
                                            e
                                        ),
                                    }
                                }
                            }
                            return Ok(Some(bssid));
                        }
                    }
                }
            }
            Err(e) => {
                log::error!("index {} Nl80211Command::GetScan error: {}", index, e);
            }
        }
    }

    log::debug!("index {} no bssid found", index);
    Ok(None)
}

/// Gets the signal strength of a wireless network connection
async fn get_signal_strength(
    socket: &NlRouter,
    index: i32,
    bssid: &MacAddr,
) -> Result<Option<SignalStrength>> {
    let mut recv = genl80211_send(
        socket,
        Nl80211Command::GetStation,
        NlmF::REQUEST,
        attrs![Ifindex => index, Mac => Buffer::from(bssid)],
    )
    .await?;

    // look for our requested data inside netlink's results
    while let Some(msg) = recv.next().await as NextNl80211 {
        match msg {
            Ok(msg) => {
                if let NlPayload::Payload(gen_msg) = msg.nl_payload() {
                    let attr_handle = gen_msg.attrs().get_attr_handle();

                    if let Ok(station_info) = attr_handle
                        .get_nested_attributes::<Nl80211StationInfo>(Nl80211Attribute::StaInfo)
                    {
                        if let Ok(signal) =
                            station_info.get_attr_payload_as::<u8>(Nl80211StationInfo::Signal)
                        {
                            let signal_strength = SignalStrength::new(signal as i8);
                            log::debug!(
                                "index {} bssid {} found signal: {} dBm",
                                index,
                                bssid,
                                signal_strength.dbm
                            );

                            return Ok(Some(signal_strength));
                        }
                    }
                }
            }
            Err(e) => {
                match e {
                    // if this error packet is returned, it means that the interface wasn't connected to the station
                    RouterError::Nlmsgerr(_) => {}
                    // any other error we should log
                    _ => {
                        log::error!(
                            "index {} bssid {} Nl80211Command::GetStation error: {}",
                            index,
                            bssid,
                            e
                        )
                    }
                }
            }
        }
    }

    log::debug!("index {} bssid {} no signal strength found", index, bssid);
    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::*;

    // signal strength tests ---------------------------------------------------

    #[test]
    fn signal_strength_quality() {
        let quality = |dbm| SignalStrength::new(dbm).quality() as u8;

        // anything at or below -110 should be 0%
        assert_eq!(0, quality(-120));
        assert_eq!(0, quality(-110));
        // lerping between -70 and 0
        assert_eq!(25, quality(-92));
        assert_eq!(50, quality(-75));
        assert_eq!(75, quality(-57));
        assert_eq!(85, quality(-50));
        // anything at or above -40 should be 100%
        assert_eq!(100, quality(-40));
        assert_eq!(100, quality(-1));
        assert_eq!(100, quality(0));
        assert_eq!(100, quality(100));
    }
}
