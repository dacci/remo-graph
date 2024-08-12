use serde::Deserialize;
use time::OffsetDateTime;

#[derive(Debug, Deserialize)]
#[allow(unused)]
pub struct DeviceResponse {
    name: String,
    id: String,
    #[serde(with = "time::serde::iso8601")]
    created_at: OffsetDateTime,
    #[serde(with = "time::serde::iso8601")]
    updated_at: OffsetDateTime,
    mac_address: String,
    bt_mac_address: Option<String>,
    serial_number: String,
    firmware_version: String,
    temperature_offset: f64,
    humidity_offset: f64,
    newest_events: NewestEvents,
    online: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct NewestEvents {
    te: Option<EventValue>,
    hu: Option<EventValue>,
    il: Option<EventValue>,
    mo: Option<EventValue>,
}

#[derive(Debug, Deserialize)]
pub struct EventValue {
    #[serde(with = "time::serde::iso8601")]
    created_at: OffsetDateTime,
    val: f64,
}

impl DeviceResponse {
    pub fn into_write_query(self: DeviceResponse) -> Vec<String> {
        let mut vec = Vec::with_capacity(4);

        if let Some(ev) = self.newest_events.te {
            vec.push(format!(
                "temperature,name={} val={} {}",
                self.name,
                ev.val,
                ev.created_at.unix_timestamp()
            ));
        }
        if let Some(ev) = self.newest_events.hu {
            vec.push(format!(
                "humidity,name={} val={} {}",
                self.name,
                ev.val,
                ev.created_at.unix_timestamp()
            ));
        }
        if let Some(ev) = self.newest_events.il {
            vec.push(format!(
                "illumination,name={} val={} {}",
                self.name,
                ev.val,
                ev.created_at.unix_timestamp()
            ));
        }
        if let Some(ev) = self.newest_events.mo {
            vec.push(format!(
                "movement,name={} val={} {}",
                self.name,
                ev.val,
                ev.created_at.unix_timestamp()
            ));
        }

        vec
    }
}
