//! Represents the DBUS API for notifications.
//! See: https://specifications.freedesktop.org/notification-spec/notification-spec-latest.html

use std::collections::HashMap;

use zbus::proxy;
use zbus::zvariant::Value;

type Hints = HashMap<&'static str, Value<'static>>;
#[proxy(
    default_path = "/org/freedesktop/Notifications",
    default_service = "org.freedesktop.Notifications",
    interface = "org.freedesktop.Notifications",
    gen_blocking = false
)]
pub trait Notifications {
    #[zbus(name = "Notify")]
    #[allow(clippy::too_many_arguments)]
    async fn notify_full(
        &self,
        app_name: &str,
        replaces_id: u32,
        app_icon: &str,
        summary: &str,
        body: &str,
        actions: &[&str],
        hints: Hints,
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

/// Easily create a hints notifications map.
macro_rules! hints {
    () => {{
        let hints: Hints = HashMap::new();
        hints
    }};

    ($($key:expr => $value:expr $(,)?)+) => {{
        let mut hints: Hints = HashMap::new();
        $(
            hints.insert($key, $value.into());
        )+

        hints

    }};
}

impl<'a> NotificationsProxy<'a> {
    const APP_NAME: &'static str = "i3stat";

    // util ----------------------------------------------------------------------------------------

    async fn notify(
        &self,
        id: Option<u32>,
        hints: Hints,
        summary: impl AsRef<str>,
        body: impl AsRef<str>,
        timeout: i32,
    ) -> Option<u32> {
        match self
            .notify_full(
                Self::APP_NAME,
                id.unwrap_or(0),
                "",
                summary.as_ref(),
                body.as_ref(),
                &[],
                hints,
                timeout,
            )
            .await
        {
            Ok(id) => Some(id),
            Err(e) => {
                log::warn!("failed to send notification: {}", e);
                id
            }
        }
    }

    async fn notify_with_stack(
        &self,
        stack_tag: &'static str,
        mut hints: Hints,
        summary: impl AsRef<str>,
        body: impl AsRef<str>,
        timeout: i32,
    ) -> Option<u32> {
        // NOTE: previously we just saved the id of the notification, and then updated that notification
        // by sending the id again. This works for dunst, but it doesn't work for mako (for mako, when the
        // original notification disappears, using the id does nothing).
        // So instead, we use the `x-dunst-stack-tag` hint which mako also supports.
        hints.insert("x-dunst-stack-tag", stack_tag.into());
        self.notify(None, hints, summary, body, timeout).await
    }

    // impl ----------------------------------------------------------------------------------------

    // NOTE: most implementations of XDG notifications over dbus expect an i32, so use that
    // (xorg/dunst seem to support using u32's, but sway/mako don't - better to go with the crowd here).
    pub async fn pulse_volume_mute(&self, name: impl AsRef<str>, pct: i32, mute: bool) {
        self.notify_with_stack(
            "pulse_volume_mute",
            hints! {
                "value" => pct,
                "urgency" => Urgency::Low,
            },
            name,
            format!("{}{}%", if mute { " " } else { " " }, pct),
            2_000,
        )
        .await;
    }

    pub async fn pulse_new_source_sink(&self, name: impl AsRef<str>, what: impl AsRef<str>) {
        self.notify(
            None,
            hints! { "urgency" => Urgency::Low },
            format!("New {} added", what.as_ref()),
            name,
            2_000,
        )
        .await;
    }

    pub async fn pulse_defaults_change(&self, name: impl AsRef<str>, what: impl AsRef<str>) {
        self.notify_with_stack(
            "pulse_defaults_change",
            hints! { "urgency" => Urgency::Low },
            format!("Default {}", what.as_ref()),
            name,
            2_000,
        )
        .await;
    }

    pub async fn ac_adapter(&self, plugged_in: bool) {
        self.notify(
            None,
            hints! { "urgency" => Urgency::Low },
            "AC Adapter",
            if plugged_in {
                "Connected"
            } else {
                "Disconnected"
            },
            2_000,
        )
        .await;
    }

    /// Trigger a critical battery charge notification that will never timeout
    pub async fn battery_critical(&self, pct: u8) {
        self.notify_with_stack(
            "battery_critical",
            hints! { "urgency" => Urgency::Critical },
            "Critical Battery Warning!",
            format!("Remaining: {}%", pct),
            // NOTE: timeout of `0` means that this notification will not go away
            0,
        )
        .await;
    }

    /// Use to disable a previously sent critical battery notification
    pub async fn battery_critical_off(&self) {
        self.notify_with_stack("battery_critical", hints! {}, "", "", 1)
            .await;
    }
}
