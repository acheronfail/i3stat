use serde_json::Value;
use zbus::dbus_proxy;

use crate::error::Result;

#[dbus_proxy(
    default_path = "/com/tuxedocomputers/tccd",
    default_service = "com.tuxedocomputers.tccd",
    interface = "com.tuxedocomputers.tccd",
    gen_blocking = false
)]
trait Tccd {
    #[dbus_proxy(name = "GetActiveProfileJSON")]
    fn get_active_profile_json(&self) -> zbus::Result<String>;

    #[dbus_proxy(name = "GetProfilesJSON")]
    fn get_profiles_json(&self) -> zbus::Result<String>;

    // FIXME: need a way to set the profile...
    // https://github.com/tuxedocomputers/tuxedo-control-center/issues/100
    // https://github.com/brunoais/tuxedo-control-center-profile-changer/discussions/2
    #[dbus_proxy(name = "SetTempProfile")]
    fn set_temp_profile(&self, profile: &str) -> zbus::Result<bool>;

    #[dbus_proxy(signal, name = "ModeReapplyPendingChanged")]
    fn mode_reapply_pending_changed(&self) -> zbus::Result<bool>;
}

#[derive(Debug)]
pub struct Profile {
    pub active: bool,
    pub name: String,
}

impl<'a> TccdProxy<'a> {
    pub async fn get_active_profile_name(&self) -> Result<String> {
        let json = self.get_active_profile_json().await?;
        let value = serde_json::from_str::<Value>(&json)?;
        let obj = match value.as_object() {
            Some(obj) => obj,
            None => bail!("TODO"),
        };

        match obj.get("name") {
            Some(Value::String(name)) => Ok(name.to_string()),
            _ => bail!("TODO"),
        }
    }

    pub async fn get_profiles(&self) -> Result<Vec<Profile>> {
        let active_name = self.get_active_profile_name().await?;

        let json = self.get_profiles_json().await?;
        let value = serde_json::from_str::<Value>(&json)?;
        let array = match value.as_array() {
            Some(profiles) => profiles,
            None => bail!("TODO"),
        };

        let mut profiles = vec![];
        for element in array {
            if let Some(obj) = element.as_object() {
                if let Some(Value::String(name)) = obj.get("name") {
                    profiles.push(Profile {
                        active: *name == active_name,
                        name: name.to_string(),
                    })
                }
            }
        }

        Ok(profiles)
    }
}
