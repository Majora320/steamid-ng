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

#[cfg(feature = "serde")]
mod serde_support;

#[macro_use]
extern crate enum_primitive;

use std::{
    error::Error,
    fmt::{self, Debug, Display, Formatter},
    str::FromStr,
};

use enum_primitive::FromPrimitive;

#[derive(Clone, Copy, PartialEq, Eq, Hash, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct SteamID(u64);

fn digit_from_ascii(byte: u8) -> Option<u8> {
    if (b'0'..=b'9').contains(&byte) {
        Some(byte - b'0')
    } else {
        None
    }
}

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

    pub fn from_steam2(steam2: &str) -> Result<Self, SteamIDParseError> {
        Self::from_steam2_helper(steam2).ok_or(SteamIDParseError {})
    }

    // Parses id in the format of:
    // ^STEAM_(universe:[0-4]):(auth_server:[0-1]):(account_id:[0-9]{1,10})$
    fn from_steam2_helper(steam2: &str) -> Option<Self> {
        let chunk = steam2.strip_prefix("STEAM_")?;
        let mut bytes = chunk.bytes();

        let mut universe: Universe = bytes
            .next()
            .and_then(|b| Universe::from_u64(u64::from(digit_from_ascii(b)?)))?;
        // Apparently, games before orange box used to display as 0 incorrectly
        // This is only an issue with steam2 ids
        if let Universe::Invalid = universe {
            universe = Universe::Public;
        }

        if bytes.next() != Some(b':') {
            return None;
        }

        let auth_server: u32 = match bytes.next()? {
            b'0' => Some(0),
            b'1' => Some(1),
            _ => None,
        }?;

        if bytes.next() != Some(b':') {
            return None;
        }

        if bytes.len() > 10 {
            return None;
        }
        let mut account_id = u32::from(digit_from_ascii(bytes.next()?)?);
        for b in bytes {
            account_id = account_id.checked_mul(10)?;
            account_id = account_id.checked_add(u32::from(digit_from_ascii(b)?))?;
        }
        let account_id = account_id << 1 | auth_server;

        Some(Self::new(
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
            AccountType::AnonGameServer | AccountType::Multiseat => render_instance = true,
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
        Self::from_steam3_helper(steam3).ok_or(SteamIDParseError {})
    }

    // Parses id in the format of:
    // ^\[(type:[AGMPCgcLTIUai]):(universe:[0-4]):(account_id:[0-9]{1,10})(:(instance:[0-9]+))?\]$
    fn from_steam3_helper(steam3: &str) -> Option<Self> {
        let mut bytes = steam3.bytes().peekable();

        if bytes.next() != Some(b'[') {
            return None;
        }

        let type_char = char::from(bytes.next()?);
        let (account_type, flag) = char_to_account_type(type_char);
        if type_char != 'i' && type_char != 'I' && account_type == AccountType::Invalid {
            return None;
        }

        if bytes.next() != Some(b':') {
            return None;
        }

        let universe = bytes.next().and_then(digit_from_ascii).and_then(|digit| {
            if digit <= 4 {
                Universe::from_u64(u64::from(digit))
            } else {
                None
            }
        })?;

        if bytes.next() != Some(b':') {
            return None;
        }

        let mut account_id = u32::from(digit_from_ascii(bytes.next()?)?);
        while let Some(digit) = bytes.peek().copied().and_then(digit_from_ascii) {
            bytes.next().expect("Byte was peeked");
            account_id = account_id.checked_mul(10)?;
            account_id = account_id.checked_add(u32::from(digit))?;
        }

        // Instance is optional. Parse it if it's there, but leave the closing ] intact
        let mut instance = {
            let maybe_instance = if bytes.peek() == Some(&b':') {
                bytes.next().expect("Byte was peeked");

                let mut acc = u64::from(digit_from_ascii(bytes.next()?)?);
                while let Some(digit) = bytes.peek().copied().and_then(digit_from_ascii) {
                    bytes.next().expect("Byte was peeked");
                    acc = acc.checked_mul(10)?;
                    acc = acc.checked_add(u64::from(digit))?;
                }

                Some(Instance::from_u64(acc).unwrap_or(Instance::Invalid))
            } else {
                None
            };

            match (maybe_instance, type_char) {
                (None, 'U') => Instance::Desktop,
                (None, _) | (_, 'T' | 'g') => Instance::All,
                (Some(instance), _) => instance,
            }
        };

        if let Some(i) = flag {
            instance = i;
        }

        if bytes.next() != Some(b']') {
            return None;
        }

        Some(Self::new(account_id, instance, account_type, universe))
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
            Result::Err(_) => match Self::from_steam2(s) {
                Ok(parsed) => Ok(parsed),
                Result::Err(_) => Self::from_steam3(s),
            },
        }
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
        AnonUser = 10,
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
        Dev = 4,
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
        FlagClan = 0x100000 >> 1,
        FlagLobby = 0x100000 >> 2,
        FlagMMSLobby = 0x100000 >> 3,
    }
);
