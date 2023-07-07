use std::collections::HashMap;

use zbus::dbus_proxy;
use zbus::zvariant::Value;

// TODO: share a single proxy instance of this, and use it wherever notifications are needed?
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
    const APP_NAME: &str = "istat";

    pub async fn pulse_volume_mute(&self, name: impl AsRef<str>, pct: u32, mute: bool) {
        let mut hints = HashMap::new();
        hints.insert("value", Value::U32(pct));
        hints.insert("urgency", Urgency::Low.into());

        if let Err(e) = self
            .notify_full(
                &format!("{}:volume", Self::APP_NAME),
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

    pub async fn pulse_new_source_sink(&self, name: impl AsRef<str>, what: impl AsRef<str>) {
        let mut hints = HashMap::new();
        hints.insert("urgency", Urgency::Low.into());

        if let Err(e) = self
            .notify_full(
                Self::APP_NAME,
                0,
                "",
                &format!("New {} added", what.as_ref()),
                name.as_ref(),
                &[],
                hints,
                2_000,
            )
            .await
        {
            log::warn!("failed to send notification: {}", e);
        }
    }

    pub async fn pulse_defaults_change(&self, name: impl AsRef<str>, what: impl AsRef<str>) {
        let mut hints = HashMap::new();
        hints.insert("urgency", Urgency::Low.into());

        if let Err(e) = self
            .notify_full(
                Self::APP_NAME,
                0,
                "",
                &format!("Default {}", what.as_ref()),
                name.as_ref(),
                &[],
                hints,
                2_000,
            )
            .await
        {
            log::warn!("failed to send notification: {}", e);
        }
    }

    pub async fn ac_adapter(&self, plugged_in: bool) {
        let mut hints = HashMap::new();
        hints.insert("urgency", Urgency::Low.into());

        if let Err(e) = self
            .notify_full(
                Self::APP_NAME,
                0,
                "",
                "AC Adapter",
                if plugged_in {
                    "Connected"
                } else {
                    "Disconnected"
                },
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
