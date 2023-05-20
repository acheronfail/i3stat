#![allow(non_snake_case)]

use zbus::dbus_proxy;
use zbus::zvariant::{DeserializeDict, OwnedObjectPath, OwnedValue, SerializeDict, Type, Value};

#[derive(Debug, DeserializeDict, SerializeDict, Value, OwnedValue, Type)]
#[zvariant(signature = "dict")]
pub struct AddressData {
    pub address: String,
    pub prefix: u32,
}

#[dbus_proxy(
    default_path = "/org/freedesktop/NetworkManager/IP4Config",
    default_service = "org.freedesktop.NetworkManager",
    interface = "org.freedesktop.NetworkManager.IP4Config",
    gen_blocking = false
)]
pub trait NetworkManagerIP4Config {
    #[dbus_proxy(property)]
    fn address_data(&self) -> zbus::Result<Vec<AddressData>>;
}

impl<'a> NetworkManagerIP4ConfigProxy<'a> {
    pub async fn extract_address(&self) -> zbus::Result<Option<String>> {
        Ok(self
            .address_data()
            .await?
            .first()
            .map(|d| d.address.to_owned()))
    }
}

#[dbus_proxy(
    default_path = "/org/freedesktop/NetworkManager/IP6Config",
    default_service = "org.freedesktop.NetworkManager",
    interface = "org.freedesktop.NetworkManager.IP6Config",
    gen_blocking = false
)]
pub trait NetworkManagerIP6Config {
    #[dbus_proxy(property)]
    fn address_data(&self) -> zbus::Result<Vec<AddressData>>;
}

impl<'a> NetworkManagerIP6ConfigProxy<'a> {
    pub async fn extract_address(&self) -> zbus::Result<Option<String>> {
        Ok(self
            .address_data()
            .await?
            .first()
            .map(|d| d.address.to_owned()))
    }
}

#[dbus_proxy(
    default_path = "/org/freedesktop/NetworkManager/ActiveConnection",
    default_service = "org.freedesktop.NetworkManager",
    interface = "org.freedesktop.NetworkManager.Connection.Active",
    gen_blocking = false
)]
trait NetworkManagerActiveConnection {
    #[dbus_proxy(property)]
    fn vpn(&self) -> zbus::Result<bool>;

    #[dbus_proxy(property, name = "Ip4Config")]
    fn ip4_config_objpath(&self) -> zbus::Result<OwnedObjectPath>;

    #[dbus_proxy(property, name = "Ip6Config")]
    fn ip6_config_objpath(&self) -> zbus::Result<OwnedObjectPath>;

    #[dbus_proxy(property, name = "Devices")]
    fn devices_objpath(&self) -> zbus::Result<Vec<OwnedObjectPath>>;

    #[dbus_proxy(property)]
    fn id(&self) -> zbus::Result<String>;

    #[dbus_proxy(property)]
    fn state(&self) -> zbus::Result<u32>;

    #[dbus_proxy(property, name = "Type")]
    fn typ(&self) -> zbus::Result<String>;
}

#[dbus_proxy(
    default_path = "/org/freedesktop/NetworkManager/Devices",
    default_service = "org.freedesktop.NetworkManager",
    interface = "org.freedesktop.NetworkManager.Device",
    gen_blocking = false
)]
pub trait NetworkManagerDevice {
    #[dbus_proxy(property)]
    fn interface(&self) -> zbus::Result<String>;

    #[dbus_proxy(property, name = "Ip4Config")]
    fn ip4_config_objpath(&self) -> zbus::Result<OwnedObjectPath>;

    #[dbus_proxy(property, name = "Ip6Config")]
    fn ip6_config_objpath(&self) -> zbus::Result<OwnedObjectPath>;

    #[dbus_proxy(property)]
    fn hw_address(&self) -> zbus::Result<String>;
}

#[dbus_proxy(
    default_path = "/org/freedesktop/NetworkManager",
    default_service = "org.freedesktop.NetworkManager",
    interface = "org.freedesktop.NetworkManager",
    gen_blocking = false
)]
pub trait NetworkManager {
    #[dbus_proxy(signal)]
    fn state_changed(&self) -> zbus::Result<()>;

    #[dbus_proxy(property, name = "ActiveConnections")]
    fn active_connections_objpath(&self) -> zbus::Result<Vec<OwnedObjectPath>>;

    #[dbus_proxy(name = "GetAllDevices")]
    fn get_all_devices_objpath(&self) -> zbus::Result<Vec<OwnedObjectPath>>;

    #[dbus_proxy(property, name = "AllDevices")]
    fn all_devices_objpath(&self) -> zbus::Result<Vec<OwnedObjectPath>>;

    #[dbus_proxy(property)]
    fn networking_enabled(&self) -> zbus::Result<bool>;

    #[dbus_proxy(property)]
    fn wireless_enabled(&self) -> zbus::Result<bool>;

    #[dbus_proxy(property)]
    fn wireless_hardware_enabled(&self) -> zbus::Result<bool>;
}

/**
 * The following macros and uses are a workaround for a limitation in zbus
 * See: https://github.com/dbus2/zbus/issues/332
 */

macro_rules! impl_object_vec {
    ($parent:ident, $child:ident, $($method:ident),+) => {
        paste::paste! {
            impl<'a> [<$parent Proxy>]<'a> {
                pub async fn [<convert_ $child:snake>](&self, paths: Vec<OwnedObjectPath>) -> zbus::Result<Vec<[<$child Proxy>]>> {
                    let list = futures::future::join_all(paths.into_iter().map(|p| async {
                        Ok::<_, zbus::Error>(
                            <[<$child Proxy>]>::builder(self.connection())
                                .path(p)?
                                .build()
                                .await?,
                        )
                    }))
                    .await
                    .into_iter()
                    .collect::<Result<Vec<_>, _>>()?;

                    Ok(list)
                }

                $(
                    pub async fn $method(&self) -> zbus::Result<Vec<[<$child Proxy>]>> {
                        let paths = self.[<$method _objpath>]().await?;
                        self.[<convert_ $child:snake>](paths).await
                    }
                )+
            }
        }
    };
}

impl_object_vec!(
    NetworkManager,
    NetworkManagerDevice,
    get_all_devices,
    all_devices
);

impl_object_vec!(
    NetworkManager,
    NetworkManagerActiveConnection,
    active_connections
);

impl_object_vec!(
    NetworkManagerActiveConnection,
    NetworkManagerDevice,
    devices
);

/**
 * This is a workaround for a limitation in zbus: the `[dbus_proxy(object = "...")]` attribute
 * only works for _methods_ not for _properties_.
 */

macro_rules! impl_object_prop {
    ($parent:ident, $child:ident, $($method:ident),+) => {
        paste::paste! {
            $(impl<'a> [<$parent Proxy>]<'a> {
                pub async fn $method(&self) -> zbus::Result<[<$child Proxy>]> {
                    let path = self.[<$method _objpath>]().await?;
                    Ok(<[<$child Proxy>]>::builder(self.connection())
                        .path(path)?
                        .build()
                        .await?)
                }
            })+
        }
    };
}

impl_object_prop!(NetworkManagerDevice, NetworkManagerIP4Config, ip4_config);
impl_object_prop!(NetworkManagerDevice, NetworkManagerIP6Config, ip6_config);
impl_object_prop!(
    NetworkManagerActiveConnection,
    NetworkManagerIP4Config,
    ip4_config
);
impl_object_prop!(
    NetworkManagerActiveConnection,
    NetworkManagerIP6Config,
    ip6_config
);
