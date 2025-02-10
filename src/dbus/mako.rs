use zbus::proxy;

#[proxy(
    default_path = "/fr/emersion/Mako",
    default_service = "org.freedesktop.Notifications",
    interface = "fr.emersion.Mako",
    gen_blocking = false
)]
pub trait Mako {
    #[zbus(property, name = "Modes")]
    fn modes(&self) -> zbus::Result<Vec<String>>;
}
