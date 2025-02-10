use zbus::proxy;

#[proxy(
    default_path = "/org/freedesktop/Notifications",
    default_service = "org.freedesktop.Notifications",
    interface = "org.dunstproject.cmd0",
    gen_blocking = false
)]
pub trait Dunst {
    #[zbus(property, name = "paused")]
    fn paused(&self) -> zbus::Result<bool>;
}
