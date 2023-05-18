use zbus::dbus_proxy;

#[dbus_proxy(
    default_path = "/org/freedesktop/Notifications",
    default_service = "org.freedesktop.Notifications",
    interface = "org.dunstproject.cmd0",
    gen_blocking = false
)]
trait Dunst {
    #[dbus_proxy(property, name = "paused")]
    fn paused(&self) -> zbus::Result<bool>;
}
