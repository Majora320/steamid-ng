use std::{
    fmt::{self, Formatter},
    str::FromStr,
};

use crate::SteamID;

use serde::{
    de::{self, Visitor},
    Deserialize, Deserializer,
};

struct SteamIDVisitor;

impl<'de> Visitor<'de> for SteamIDVisitor {
    type Value = SteamID;

    fn expecting(&self, formatter: &mut Formatter) -> fmt::Result {
        formatter.write_str("a SteamID")
    }

    fn visit_str<E>(self, value: &str) -> Result<SteamID, E>
    where
        E: de::Error,
    {
        SteamID::from_str(value).map_err(|_| E::custom(format!("Invalid SteamID: {}", value)))
    }

    fn visit_u64<E>(self, value: u64) -> Result<SteamID, E>
    where
        E: de::Error,
    {
        Ok(value.into())
    }
}

impl<'de> Deserialize<'de> for SteamID {
    fn deserialize<D>(deserializer: D) -> Result<SteamID, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_any(SteamIDVisitor)
    }
}
