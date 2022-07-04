#[cfg(feature = "serde")]
use steamid_ng::{AccountType, Instance, SteamID, Universe};

#[test]
#[cfg_attr(not(feature = "serde"), ignore)]
fn test_serde() {
    #[cfg(not(feature = "serde"))]
    {
        panic!("Test only enabled with the 'serde' feature");
    }
    #[cfg(feature = "serde")]
    {
        let s = SteamID::new(1234, Instance::Console, AccountType::Chat, Universe::Beta);
        let serialized: String = serde_json::to_string(&s).unwrap();
        let deserialized: SteamID = serde_json::from_str(&serialized).unwrap();
        assert_eq!(s, deserialized);

        let deserialized: SteamID = serde_json::from_str("\"STEAM_0:0:4491990\"").unwrap();
        // Prevent rustfmt bug where it adds an extra comma even though assert_eq! is a macro
        #[cfg_attr(rustfmt, rustfmt_skip)]
            assert_eq!(
                deserialized,
                SteamID::new(
                    8983980,
                    Instance::Desktop,
                    AccountType::Individual,
                    Universe::Public,
                )
            );

        let deserialized: SteamID = serde_json::from_str("\"[U:1:123]\"").unwrap();
        #[cfg_attr(rustfmt, rustfmt_skip)]
            assert_eq!(
                deserialized,
                SteamID::new(
                    123,
                    Instance::Desktop,
                    AccountType::Individual,
                    Universe::Public,
                )
            );

        let deserialized: SteamID = serde_json::from_str("103582791432294076").unwrap();
        #[cfg_attr(rustfmt, rustfmt_skip)]
            assert_eq!(
                deserialized,
                SteamID::new(2772668, Instance::All, AccountType::Clan, Universe::Public)
            );

        let serialized: String = serde_json::to_string(&SteamID::new(
            8983981,
            Instance::Desktop,
            AccountType::Individual,
            Universe::Public,
        ))
        .unwrap();
        assert_eq!(serialized, "76561197969249709");

        let serialized: String = serde_json::to_string(&SteamID::new(
            123,
            Instance::Web,
            AccountType::AnonGameServer,
            Universe::Public,
        ))
        .unwrap();
        assert_eq!(serialized, "90072009727279227");
    }
}
