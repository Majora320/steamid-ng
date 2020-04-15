//! # SteamID
//! The steamid-ng crate provides an easy-to-use SteamID type with functions to parse and render
//! steam2 and steam3 IDs. It also supports serializing and deserializing via
//! [serde](https://serde.rs).
//!
//! ## Examples
//!
//! ```
//! # use steamid_ng::{SteamID, Instance, AccountType, Universe};
//! let x = SteamID::from(76561197960287930);
//! let y = SteamID::from_steam3("[U:1:22202]").unwrap();
//! let z = SteamID::from_steam2("STEAM_1:0:11101").unwrap();
//! assert_eq!(x, y);
//! assert_eq!(y, z);
//!
//! assert_eq!(u64::from(z), 76561197960287930);
//! assert_eq!(y.steam2(), "STEAM_1:0:11101");
//! assert_eq!(x.steam3(), "[U:1:22202]");
//!
//! assert_eq!(x.account_id(), 22202);
//! assert_eq!(x.instance(), Instance::Desktop);
//! assert_eq!(x.account_type(), AccountType::Individual);
//! assert_eq!(x.universe(), Universe::Public);
//! // the SteamID type also has `set_{account_id, instance, account_type, universe}` methods,
//! // which work as you would expect.
//! ```
//!
//! Keep in mind that the SteamID type does no validation.

#[macro_use]
extern crate enum_primitive;
#[macro_use]
extern crate serde_derive;
#[macro_use]
extern crate lazy_static;
extern crate regex;
extern crate serde;

use serde::de::{self, Visitor, Deserialize, Deserializer};
use enum_primitive::FromPrimitive;
use std::fmt::{self, Debug, Display};
use std::fmt::Formatter;
use std::str::FromStr;
use regex::Regex;
use std::error::Error;

#[derive(Clone, Copy, PartialEq, Eq, Hash, Default, Serialize)]
pub struct SteamID(u64);

impl SteamID {
    pub fn account_id(&self) -> u32 {
        // only ever 32 bits
        (self.0 & 0xFFFFFFFF) as u32
    }

    pub fn set_account_id(&mut self, account_id: u32) {
        self.0 &= 0xFFFFFFFF00000000;
        self.0 |= u64::from(account_id);
    }

    pub fn instance(&self) -> Instance {
        Instance::from_u64((self.0 >> 32) & 0xFFFFF).unwrap_or(Instance::Invalid)
    }

    pub fn set_instance(&mut self, instance: Instance) {
        self.0 &= 0xFFF00000FFFFFFFF;
        self.0 |= (instance as u64) << 32;
    }

    pub fn account_type(&self) -> AccountType {
        AccountType::from_u64((self.0 >> 52) & 0xF).unwrap_or(AccountType::Invalid)
    }

    pub fn set_account_type(&mut self, account_type: AccountType) {
        self.0 &= 0xFF0FFFFFFFFFFFFF;
        self.0 |= (account_type as u64) << 52;
    }

    pub fn universe(&self) -> Universe {
        Universe::from_u64((self.0 >> 56) & 0xFF).unwrap_or(Universe::Invalid)
    }

    pub fn set_universe(&mut self, universe: Universe) {
        self.0 &= 0x00FFFFFFFFFFFFFF;
        self.0 |= (universe as u64) << 56;
    }

    pub fn new(
        account_id: u32,
        instance: Instance,
        account_type: AccountType,
        universe: Universe,
    ) -> Self {
        #[cfg_attr(rustfmt, rustfmt_skip)]
        Self::from(
            u64::from(account_id)         | ((instance as u64) << 32) |
            ((account_type as u64) << 52) | ((universe as u64) << 56),
        )
    }

    pub fn steam2(&self) -> String {
        match self.account_type() {
            AccountType::Individual | AccountType::Invalid => {
                let id = self.account_id();
                format!("STEAM_{}:{}:{}", self.universe() as u64, id & 1, id >> 1)
            }
            _ => self.0.to_string(),
        }
    }

    /// Little ergonomics thing, to avoid typing `SteamIDParseError::default()` everywhere
    fn err() -> SteamIDParseError {
        Default::default()
    }

    pub fn from_steam2(steam2: &str) -> Result<Self, SteamIDParseError> {
        lazy_static! {
            static ref STEAM2_REGEX: Regex =
                Regex::new(r"^STEAM_([0-4]):([0-1]):([0-9]{1,10})$").unwrap();
        }

        let groups = STEAM2_REGEX.captures(steam2).ok_or_else(Self::err)?;

        let mut universe: Universe = Universe::from_u64(
            groups.get(1).ok_or_else(Self::err)?.as_str().parse().unwrap(),
        ).ok_or_else(Self::err)?;
        let auth_server: u32 = groups.get(2).ok_or_else(Self::err)?.as_str().parse().unwrap();
        #[cfg_attr(rustfmt, rustfmt_skip)]
        let account_id: u32 = groups.get(3).ok_or_else(Self::err)?.as_str().parse().unwrap();
        let account_id = account_id << 1 | auth_server;

        // Apparently, games before orange box used to display as 0 incorrectly
        // This is only an issue with steam2 ids
        if let Universe::Invalid = universe {
            universe = Universe::Public;
        }

        Ok(Self::new(
            account_id,
            Instance::Desktop,
            AccountType::Individual,
            universe,
        ))
    }

    pub fn steam3(&self) -> String {
        let instance = self.instance();
        let account_type = self.account_type();
        let mut render_instance = false;

        match account_type {
            AccountType::AnonGameServer |
            AccountType::Multiseat => render_instance = true,
            AccountType::Individual => render_instance = instance != Instance::Desktop,
            _ => (),
        };

        if render_instance {
            format!(
                "[{}:{}:{}:{}]",
                account_type_to_char(account_type, instance),
                self.universe() as u64,
                self.account_id(),
                instance as u64
            )
        } else {
            format!(
                "[{}:{}:{}]",
                account_type_to_char(account_type, instance),
                self.universe() as u64,
                self.account_id()
            )
        }
    }

    pub fn from_steam3(steam3: &str) -> Result<Self, SteamIDParseError> {
        lazy_static! {
            static ref STEAM3_REGEX: Regex =
                Regex::new(r"^\[([AGMPCgcLTIUai]):([0-4]):([0-9]{1,10})(:([0-9]+))?\]$").unwrap();
        }

        let groups = STEAM3_REGEX.captures(steam3).ok_or_else(Self::err)?;

        let type_char = groups
            .get(1)
            .ok_or_else(Self::err)?
            .as_str()
            .chars()
            .next()
            .unwrap();
        let (account_type, flag) = char_to_account_type(type_char);
        let universe = Universe::from_u64(
            groups.get(2).ok_or_else(Self::err)?.as_str().parse().unwrap(),
        ).ok_or_else(Self::err)?;
        let account_id = groups.get(3).ok_or_else(Self::err)?.as_str().parse().unwrap();

        let mut instance: Option<Instance> = groups.get(5).map(|g| {
            Instance::from_u64(g.as_str().parse().unwrap()).unwrap_or(Instance::Invalid)
        });

        if instance.is_none() && type_char == 'U' {
            instance = Some(Instance::Desktop);
        } else if type_char == 'T' || type_char == 'g' || instance.is_none() {
            instance = Some(Instance::All);
        }

        if let Some(i) = flag {
            instance = Some(i);
        }

        Ok(Self::new(
            account_id,
            instance.ok_or_else(Self::err)?,
            account_type,
            universe,
        ))
    }
}

#[derive(Copy, Clone, Debug, Hash, PartialEq, Eq, PartialOrd, Ord, Default)]
pub struct SteamIDParseError {}

impl Error for SteamIDParseError {}

impl Display for SteamIDParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Malformed SteamID")
    }
}

impl From<u64> for SteamID {
    fn from(s: u64) -> Self {
        SteamID(s)
    }
}

impl From<SteamID> for u64 {
    fn from(s: SteamID) -> Self {
        s.0
    }
}

impl From<SteamID> for String {
    /// Returns a Steam3 representation of the SteamID
    fn from(s: SteamID) -> Self {
        s.steam3()
    }
}

// TODO: convert this to TryFrom once it's out of nightly
// There will probably be a blanket impl that provides FromStr automatically
impl FromStr for SteamID {
    type Err = SteamIDParseError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.parse::<u64>() {
            Ok(parsed) => Ok(parsed.into()),
            Result::Err(_) => {
                match Self::from_steam2(s) {
                    Ok(parsed) => Ok(parsed),
                    Result::Err(_) => Self::from_steam3(s),
                }
            }
        }
    }
}

pub struct SteamIDVisitor;
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

impl Debug for SteamID {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(
            f,
            "SteamID({}) {{ID: {}, Instance: {:?}, Type: {:?}, Universe: {:?}}}",
            self.0,
            self.account_id(),
            self.instance(),
            self.account_type(),
            self.universe()
        )
    }
}

enum_from_primitive!(
    #[derive(Copy, Clone, PartialEq, Eq, Debug)]
    pub enum AccountType {
        Invalid = 0,
        Individual = 1,
        Multiseat = 2,
        GameServer = 3,
        AnonGameServer = 4,
        Pending = 5,
        ContentServer = 6,
        Clan = 7,
        Chat = 8,
        P2PSuperSeeder = 9,
        AnonUser = 10
    }
);

pub fn account_type_to_char(account_type: AccountType, instance: Instance) -> char {
    match account_type {
        AccountType::Invalid => 'I',
        AccountType::Individual => 'U',
        AccountType::Multiseat => 'M',
        AccountType::GameServer => 'G',
        AccountType::AnonGameServer => 'A',
        AccountType::Pending => 'P',
        AccountType::ContentServer => 'C',
        AccountType::Clan => 'g',
        AccountType::Chat => {
            if let Instance::FlagClan = instance {
                'c'
            } else if let Instance::FlagLobby = instance {
                'L'
            } else {
                'T'
            }
        }
        AccountType::AnonUser => 'a',
        AccountType::P2PSuperSeeder => 'i', // Invalid (?)
    }
}

/// In certain cases, this function will return an Instance as the second item in the tuple. You
/// should set the instance of the underlying SteamID to this value.
pub fn char_to_account_type(c: char) -> (AccountType, Option<Instance>) {
    match c {
        'U' => (AccountType::Individual, None),
        'M' => (AccountType::Multiseat, None),
        'G' => (AccountType::GameServer, None),
        'A' => (AccountType::AnonGameServer, None),
        'P' => (AccountType::Pending, None),
        'C' => (AccountType::ContentServer, None),
        'g' => (AccountType::Clan, None),

        'T' => (AccountType::Chat, None),
        'c' => (AccountType::Chat, Some(Instance::FlagClan)),
        'L' => (AccountType::Chat, Some(Instance::FlagLobby)),

        'a' => (AccountType::AnonUser, None),

        'I' | 'i' | _ => (AccountType::Invalid, None),
    }
}

enum_from_primitive!(
    #[derive(Copy, Clone, PartialEq, Eq, Debug)]
    pub enum Universe {
        Invalid = 0,
        Public = 1,
        Beta = 2,
        Internal = 3,
        Dev = 4
    }
);

enum_from_primitive!(
    #[derive(Copy, Clone, PartialEq, Eq, Debug)]
    pub enum Instance {
        All = 0,
        Desktop = 1,
        Console = 2,
        Web = 4,
        // Made up magic constant
        Invalid = 666,
        // *Apparently*, All will by the only type used if any of these is set
        FlagClan =     0x100000 >> 1,
        FlagLobby =    0x100000 >> 2,
        FlagMMSLobby = 0x100000 >> 3
    }
);
