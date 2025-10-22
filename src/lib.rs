//! The steamid-ng crate provides an easy-to-use [`SteamID`] type with functions to parse and render
//! steam2 and steam3 IDs. It also supports serializing and deserializing via [serde](https://serde.rs).
//!
//! # Examples
//!
//! ```
//! # use steamid_ng::*;
//! let x = SteamID::from_steam64(76561197960287930).unwrap();
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
//! assert_eq!(x.instance().instance_type(), InstanceType::Desktop);
//! assert_eq!(x.account_type(), AccountType::Individual);
//! assert_eq!(x.universe(), Universe::Public);
//! // the SteamID type also has `set_{account_id, instance, account_type, universe}` methods,
//! // which work as you would expect.
//! ```
//!
//! All constructed SteamID types are valid Steam IDs; values provided will be validated in all cases.
//! If an ID provided by an official Valve service fails to parse, that should be considered a bug
//! in this library, and you should open an issue [on GitHub](https://github.com/Majora320/steamid-ng/issues).

#[cfg(feature = "serde")]
use serde::{
    Deserialize, Deserializer, Serialize,
    de::{self, Visitor},
};
use std::{
    error::Error,
    fmt::{self, Debug, Display, Formatter},
    str::FromStr,
};

#[derive(Copy, Clone, PartialEq, Eq, Debug, Default)]
pub struct SteamIDParseError;

impl Error for SteamIDParseError {}

impl Display for SteamIDParseError {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "Malformed SteamID")
    }
}

fn digit_from_ascii(byte: u8) -> Option<u8> {
    if byte.is_ascii_digit() {
        Some(byte - b'0')
    } else {
        None
    }
}

#[cfg_attr(feature = "serde", derive(Serialize))]
#[derive(Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct SteamID(u64);

impl SteamID {
    pub fn new(
        account_id: u32,
        instance: Instance,
        account_type: AccountType,
        universe: Universe,
    ) -> Self {
        Self(
            (account_id as u64)
                | ((instance.0 as u64) << 32)
                | ((account_type as u64) << 52)
                | ((universe as u64) << 56),
        )
    }

    pub fn account_id(&self) -> u32 {
        (self.0 & 0xFFFFFFFF) as u32
    }

    pub fn set_account_id(&mut self, account_id: u32) {
        self.0 &= 0xFFFFFFFF00000000;
        self.0 |= account_id as u64;
    }

    pub fn instance(&self) -> Instance {
        Instance::try_from(((self.0 >> 32) & 0xFFFFF) as u32).expect("Instance should be valid")
    }

    pub fn set_instance(&mut self, instance: Instance) {
        self.0 &= 0xFFF00000FFFFFFFF;
        self.0 |= (instance.0 as u64) << 32;
    }

    pub fn set_instance_type(&mut self, instance_type: InstanceType) {
        let mut instance = self.instance();
        instance.set_instance_type(instance_type);
        self.set_instance(instance);
    }

    pub fn set_instance_flags(&mut self, instance_flags: InstanceFlags) {
        let mut instance = self.instance();
        instance.set_flags(instance_flags);
        self.set_instance(instance);
    }

    pub fn account_type(&self) -> AccountType {
        AccountType::try_from(((self.0 >> 52) & 0xF) as u8).expect("Account type should be valid")
    }

    pub fn set_account_type(&mut self, account_type: AccountType) {
        self.0 &= 0xFF0FFFFFFFFFFFFF;
        self.0 |= (account_type as u64) << 52;
    }

    pub fn universe(&self) -> Universe {
        Universe::try_from(((self.0 >> 56) & 0xFF) as u8).expect("Universe should be valid")
    }

    pub fn set_universe(&mut self, universe: Universe) {
        self.0 &= 0x00FFFFFFFFFFFFFF;
        self.0 |= (universe as u64) << 56;
    }

    pub fn steam64(&self) -> u64 {
        self.0
    }

    pub fn from_steam64(value: u64) -> Result<Self, SteamIDParseError> {
        Self::try_from(value)
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

    // Parses id in the format of:
    // ^STEAM_(universe:[0-4]):(auth_server:[0-1]):(account_id:[0-9]{1,10})$
    pub fn from_steam2(steam2: &str) -> Result<Self, SteamIDParseError> {
        let chunk = steam2.strip_prefix("STEAM_").ok_or(SteamIDParseError)?;
        let mut bytes = chunk.bytes();

        let mut universe: Universe = bytes
            .next()
            .and_then(digit_from_ascii)
            .ok_or(SteamIDParseError)
            .and_then(Universe::try_from)?;
        // Apparently, games before orange box used to display as 0 incorrectly
        // This is only an issue with steam2 ids
        if let Universe::Invalid = universe {
            universe = Universe::Public;
        }

        if bytes.next() != Some(b':') {
            return Err(SteamIDParseError);
        }

        let auth_server: u32 = match bytes.next().ok_or(SteamIDParseError)? {
            b'0' => Ok(0),
            b'1' => Ok(1),
            _ => Err(SteamIDParseError),
        }?;

        if bytes.next() != Some(b':') {
            return Err(SteamIDParseError);
        }

        if bytes.len() > 10 {
            return Err(SteamIDParseError);
        }

        let mut account_id = bytes
            .next()
            .and_then(digit_from_ascii)
            .ok_or(SteamIDParseError)? as u32;
        for b in bytes {
            let digit = digit_from_ascii(b).ok_or(SteamIDParseError)? as u32;
            account_id = account_id
                .checked_mul(10)
                .and_then(|id| id.checked_add(digit))
                .ok_or(SteamIDParseError)?;
        }
        let account_id = account_id << 1 | auth_server;

        Ok(Self::new(
            account_id,
            Instance::new(InstanceType::Desktop, InstanceFlags::None),
            AccountType::Individual,
            universe,
        ))
    }

    pub fn steam3(&self) -> String {
        let account_type = self.account_type();
        let instance = self.instance();
        let instance_type = instance.instance_type();
        let instance_flags = instance.flags();
        let mut render_instance = false;

        match account_type {
            AccountType::AnonGameServer | AccountType::Multiseat => render_instance = true,
            AccountType::Individual => render_instance = instance_type != InstanceType::Desktop,
            _ => (),
        };

        if render_instance {
            format!(
                "[{}:{}:{}:{}]",
                account_type_to_char(account_type, instance_flags),
                self.universe() as u64,
                self.account_id(),
                instance.0
            )
        } else {
            format!(
                "[{}:{}:{}]",
                account_type_to_char(account_type, instance_flags),
                self.universe() as u64,
                self.account_id()
            )
        }
    }

    // Parses id in the format of:
    // ^\[(type:[AGMPCgcLTIUai]):(universe:[0-4]):(account_id:[0-9]{1,10})(:(instance:[0-9]+))?\]$
    pub fn from_steam3(steam3: &str) -> Result<Self, SteamIDParseError> {
        let mut bytes = steam3.bytes().peekable();

        if bytes.next() != Some(b'[') {
            return Err(SteamIDParseError);
        }

        let (account_type, instance_flags) = bytes
            .next()
            .and_then(|b| char_to_account_type(b.into()))
            .ok_or(SteamIDParseError)?;

        if bytes.next() != Some(b':') {
            return Err(SteamIDParseError);
        }

        let universe = bytes
            .next()
            .and_then(digit_from_ascii)
            .ok_or(SteamIDParseError)
            .and_then(Universe::try_from)?;

        if bytes.next() != Some(b':') {
            return Err(SteamIDParseError);
        }

        let mut account_id = bytes
            .next()
            .and_then(digit_from_ascii)
            .ok_or(SteamIDParseError)? as u32;
        while let Some(digit) = bytes.peek().copied().and_then(digit_from_ascii) {
            bytes.next().expect("Byte was peeked");
            account_id = account_id
                .checked_mul(10)
                .and_then(|id| id.checked_add(digit as u32))
                .ok_or(SteamIDParseError)?;
        }

        // Instance is optional. Parse it if it's there, but leave the closing ] intact
        let instance_type = {
            let maybe_instance_type = if bytes.peek().copied() == Some(b':') {
                bytes.next().expect("Byte was peeked");

                let mut acc = bytes
                    .next()
                    .and_then(digit_from_ascii)
                    .ok_or(SteamIDParseError)? as u32;
                while let Some(digit) = bytes.peek().copied().and_then(digit_from_ascii) {
                    bytes.next().expect("Byte was peeked");
                    acc = acc
                        .checked_mul(10)
                        .and_then(|id| id.checked_add(digit as u32))
                        .ok_or(SteamIDParseError)?;
                }

                Some(InstanceType::try_from(acc)?)
            } else {
                None
            };

            match (maybe_instance_type, account_type) {
                (None, AccountType::Individual) => InstanceType::Desktop,
                (None, _) => InstanceType::All,
                (Some(_), AccountType::Clan | AccountType::Chat) => InstanceType::All,
                (Some(instance_type), _) => instance_type,
            }
        };

        if bytes.next() != Some(b']') || bytes.next().is_some() {
            return Err(SteamIDParseError);
        }

        Ok(Self::new(
            account_id,
            Instance::new(instance_type, instance_flags),
            account_type,
            universe,
        ))
    }
}

impl TryFrom<u64> for SteamID {
    type Error = SteamIDParseError;

    fn try_from(value: u64) -> Result<Self, Self::Error> {
        Instance::try_from((value >> 32 & 0xFFFFF) as u32)?;
        AccountType::try_from((value >> 52 & 0xF) as u8)?;
        Universe::try_from((value >> 56 & 0xFF) as u8)?;

        Ok(SteamID(value))
    }
}

impl From<SteamID> for u64 {
    fn from(s: SteamID) -> Self {
        s.0
    }
}

impl FromStr for SteamID {
    type Err = SteamIDParseError;
    /// Tries to parse the given string as all three types of SteamID, and returns an error if
    /// all three attempts fail. You should use [`SteamID::from_steam3`] or [`SteamID::from_steam2`]
    /// if you know the format of the SteamID string you are trying to parse.
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if let Ok(u) = s.parse::<u64>() {
            SteamID::try_from(u)
        } else if let Ok(s) = Self::from_steam2(s) {
            Ok(s)
        } else if let Ok(s) = Self::from_steam3(s) {
            Ok(s)
        } else {
            Err(SteamIDParseError)
        }
    }
}

#[cfg(feature = "serde")]
struct SteamIDVisitor;
#[cfg(feature = "serde")]
impl<'de> Visitor<'de> for SteamIDVisitor {
    type Value = SteamID;

    fn expecting(&self, formatter: &mut Formatter) -> fmt::Result {
        formatter.write_str("a SteamID")
    }

    fn visit_u64<E>(self, value: u64) -> Result<SteamID, E>
    where
        E: de::Error,
    {
        SteamID::try_from(value).map_err(|_| E::custom(format!("invalid SteamID: {}", value)))
    }

    fn visit_str<E>(self, value: &str) -> Result<SteamID, E>
    where
        E: de::Error,
    {
        SteamID::from_str(value).map_err(|_| E::custom(format!("invalid SteamID: {}", value)))
    }
}

#[cfg(feature = "serde")]
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

#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
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
    ConsoleUser = 9,
    AnonUser = 10,
}

impl TryFrom<u8> for AccountType {
    type Error = SteamIDParseError;
    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(AccountType::Invalid),
            1 => Ok(AccountType::Individual),
            2 => Ok(AccountType::Multiseat),
            3 => Ok(AccountType::GameServer),
            4 => Ok(AccountType::AnonGameServer),
            5 => Ok(AccountType::Pending),
            6 => Ok(AccountType::ContentServer),
            7 => Ok(AccountType::Clan),
            8 => Ok(AccountType::Chat),
            9 => Ok(AccountType::ConsoleUser),
            10 => Ok(AccountType::AnonUser),
            _ => Err(SteamIDParseError),
        }
    }
}

pub fn account_type_to_char(account_type: AccountType, flags: InstanceFlags) -> char {
    match account_type {
        AccountType::Invalid => 'I',
        AccountType::Individual => 'U',
        AccountType::Multiseat => 'M',
        AccountType::GameServer => 'G',
        AccountType::AnonGameServer => 'A',
        AccountType::Pending => 'P',
        AccountType::ContentServer => 'C',
        AccountType::Clan => 'g',
        AccountType::Chat => match flags {
            InstanceFlags::Clan => 'c',
            InstanceFlags::Lobby => 'L',
            _ => 'T',
        },
        AccountType::ConsoleUser => 'U',
        AccountType::AnonUser => 'a',
    }
}

pub fn char_to_account_type(c: char) -> Option<(AccountType, InstanceFlags)> {
    match c {
        'U' => Some((AccountType::Individual, InstanceFlags::None)),
        'M' => Some((AccountType::Multiseat, InstanceFlags::None)),
        'G' => Some((AccountType::GameServer, InstanceFlags::None)),
        'A' => Some((AccountType::AnonGameServer, InstanceFlags::None)),
        'P' => Some((AccountType::Pending, InstanceFlags::None)),
        'C' => Some((AccountType::ContentServer, InstanceFlags::None)),
        'g' => Some((AccountType::Clan, InstanceFlags::None)),
        'T' => Some((AccountType::Chat, InstanceFlags::None)),
        'c' => Some((AccountType::Chat, InstanceFlags::Clan)),
        'L' => Some((AccountType::Chat, InstanceFlags::Lobby)),
        'a' => Some((AccountType::AnonUser, InstanceFlags::None)),
        'I' | 'i' => Some((AccountType::Invalid, InstanceFlags::None)),
        _ => None,
    }
}

#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub enum Universe {
    Invalid = 0,
    Public = 1,
    Beta = 2,
    Internal = 3,
    Dev = 4,
    RC = 5,
}

impl TryFrom<u8> for Universe {
    type Error = SteamIDParseError;
    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Universe::Invalid),
            1 => Ok(Universe::Public),
            2 => Ok(Universe::Beta),
            3 => Ok(Universe::Internal),
            4 => Ok(Universe::Dev),
            5 => Ok(Universe::RC),
            _ => Err(SteamIDParseError),
        }
    }
}

#[derive(Copy, Clone, PartialEq, Eq, Hash)]
pub struct Instance(u32);

impl Instance {
    pub fn new(instance_type: InstanceType, flags: InstanceFlags) -> Self {
        Instance(instance_type as u32 | (flags as u32) << 12)
    }

    pub fn instance_type(&self) -> InstanceType {
        match self.0 & 0xFFF {
            0 => InstanceType::All,
            1 => InstanceType::Desktop,
            2 => InstanceType::Console,
            4 => InstanceType::Web,
            _ => unreachable!(),
        }
    }

    pub fn set_instance_type(&mut self, instance_type: InstanceType) {
        self.0 &= 0xFF000;
        self.0 |= instance_type as u32;
    }

    pub fn flags(&self) -> InstanceFlags {
        match self.0 >> 12 {
            0 => InstanceFlags::None,
            0b1000_0000 => InstanceFlags::Clan,
            0b0100_0000 => InstanceFlags::Lobby,
            0b0010_0000 => InstanceFlags::MMSLobby,
            _ => unreachable!(),
        }
    }

    pub fn set_flags(&mut self, flags: InstanceFlags) {
        self.0 &= 0x00FFF;
        self.0 |= (flags as u32) << 12;
    }
}

impl Debug for Instance {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(
            f,
            "{{Type: {:?}, Flags: {:?}}}",
            self.instance_type(),
            self.flags()
        )
    }
}

impl TryFrom<u32> for Instance {
    type Error = SteamIDParseError;
    fn try_from(value: u32) -> Result<Self, Self::Error> {
        InstanceType::try_from(value & 0xFFF)?;
        InstanceFlags::try_from((value >> 12) as u8)?;
        Ok(Instance(value))
    }
}

#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub enum InstanceType {
    All = 0,
    Desktop = 1,
    Console = 2,
    Web = 4,
}

impl TryFrom<u32> for InstanceType {
    type Error = SteamIDParseError;
    fn try_from(value: u32) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(InstanceType::All),
            1 => Ok(InstanceType::Desktop),
            2 => Ok(InstanceType::Console),
            4 => Ok(InstanceType::Web),
            _ => Err(SteamIDParseError),
        }
    }
}

#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug, Default)]
pub enum InstanceFlags {
    #[default]
    None = 0,
    Clan = 0b1000_0000,
    Lobby = 0b0100_0000,
    MMSLobby = 0b0010_0000,
}

impl TryFrom<u8> for InstanceFlags {
    type Error = SteamIDParseError;
    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(InstanceFlags::None),
            0b1000_0000 => Ok(InstanceFlags::Clan),
            0b0100_0000 => Ok(InstanceFlags::Lobby),
            0b0010_0000 => Ok(InstanceFlags::MMSLobby),
            _ => Err(SteamIDParseError),
        }
    }
}
