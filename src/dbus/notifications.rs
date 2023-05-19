use std::collections::HashMap;

use zbus::dbus_proxy;
use zbus::zvariant::Value;

#[dbus_proxy(
    default_path = "/org/freedesktop/Notifications",
    default_service = "org.freedesktop.Notifications",
    interface = "org.freedesktop.Notifications",
    gen_blocking = false
)]
trait Notifications {
    // See: https://specifications.freedesktop.org/notification-spec/notification-spec-latest.html
    #[dbus_proxy(name = "Notify")]
    fn notify_full(
        &self,
        app_name: &str,
        replaces_id: u32,
        app_icon: &str,
        summary: &str,
        body: &str,
        actions: &[&str],
        hints: HashMap<&str, Value<'_>>,
        expire_timeout: i32,
    ) -> zbus::Result<u32>;
}

#[derive(Debug)]
pub enum Urgency {
    Low = 0,
    Normal = 1,
    Critical = 2,
}

impl<'a> From<Urgency> for Value<'a> {
    fn from(value: Urgency) -> Self {
        Value::U8(value as u8)
    }
}

impl<'a> NotificationsProxy<'a> {
    pub async fn volume(&self, name: impl AsRef<str>, pct: u32, mute: bool) {
        let mut hints = HashMap::new();
        hints.insert("value", Value::U32(pct));
        hints.insert("urgency", Urgency::Low.into());

        if let Err(e) = self
            .notify_full(
                "staturs",
                0,
                // TODO: icon
                "audio-card",
                name.as_ref(),
                // TODO: better muted state (notification icon or more obvious in notification)
                &format!("{}{}%", if mute { " " } else { " " }, pct),
                &[],
                hints,
                2_000,
            )
            .await
        {
            log::warn!("failed to send notification: {}", e);
        }
    }
}
