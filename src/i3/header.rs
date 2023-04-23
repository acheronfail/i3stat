use serde_derive::Serialize;

#[derive(Debug, Serialize)]
pub struct I3BarHeader {
    version: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    stop_signal: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    cont_signal: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    click_events: Option<bool>,
}

impl Default for I3BarHeader {
    fn default() -> Self {
        I3BarHeader {
            version: 1,
            stop_signal: None,
            cont_signal: None,
            click_events: Some(true),
        }
    }
}
