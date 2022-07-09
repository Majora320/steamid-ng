use steamid_ng::*;

#[test]
fn test_manual_construction() {
    let mut s = SteamID::from(0);
    s.set_account_id(1234);
    s.set_instance(Instance::Console);
    s.set_account_type(AccountType::Chat);
    s.set_universe(Universe::Beta);

    assert_eq!(s.account_id(), 1234);
    assert_eq!(s.instance(), Instance::Console);
    assert_eq!(s.account_type(), AccountType::Chat);
    assert_eq!(s.universe(), Universe::Beta);
    assert_eq!(
        s,
        SteamID::new(1234, Instance::Console, AccountType::Chat, Universe::Beta)
    );

    s.set_account_id(4567);
    assert_eq!(s.account_id(), 4567);
    assert_eq!(s.instance(), Instance::Console);
    assert_eq!(s.account_type(), AccountType::Chat);
    assert_eq!(s.universe(), Universe::Beta);

    s.set_universe(Universe::Dev);
    assert_eq!(s.account_id(), 4567);
    assert_eq!(s.instance(), Instance::Console);
    assert_eq!(s.account_type(), AccountType::Chat);
    assert_eq!(s.universe(), Universe::Dev);

    s.set_instance(Instance::Web);
    assert_eq!(s.account_id(), 4567);
    assert_eq!(s.instance(), Instance::Web);
    assert_eq!(s.account_type(), AccountType::Chat);
    assert_eq!(s.universe(), Universe::Dev);

    s.set_account_type(AccountType::GameServer);
    assert_eq!(s.account_id(), 4567);
    assert_eq!(s.instance(), Instance::Web);
    assert_eq!(s.account_type(), AccountType::GameServer);
    assert_eq!(s.universe(), Universe::Dev);
}

#[test]
fn test_from_u64() {
    let s = SteamID::from(103582791432294076);
    assert_eq!(s.account_id(), 2772668);
    assert_eq!(s.instance(), Instance::All);
    assert_eq!(s.account_type(), AccountType::Clan);
    assert_eq!(s.universe(), Universe::Public);

    let s = SteamID::from(157626004137848889);
    assert_eq!(s.account_id(), 12345);
    assert_eq!(s.instance(), Instance::Web);
    assert_eq!(s.account_type(), AccountType::GameServer);
    assert_eq!(s.universe(), Universe::Beta);
}

#[test]
fn test_steam2() {
    let mut s = SteamID::from(76561197969249708);

    assert_eq!(s.steam2(), "STEAM_1:0:4491990");
    s.set_universe(Universe::Invalid);
    assert_eq!(s.steam2(), "STEAM_0:0:4491990");
    s.set_universe(Universe::Beta);
    assert_eq!(s.steam2(), "STEAM_2:0:4491990");
    s.set_account_type(AccountType::GameServer);
    assert_eq!(s.steam2(), "157625991261918636");
}

#[test]
fn test_from_steam2() {
    let s = SteamID::from_steam2("STEAM_0:0:4491990").unwrap();
    assert_eq!(s.account_id(), 8983980);
    assert_eq!(s.instance(), Instance::Desktop);
    assert_eq!(s.account_type(), AccountType::Individual);
    assert_eq!(s.universe(), Universe::Public);

    let s = SteamID::from_steam2("STEAM_0:1:4491990").unwrap();
    assert_eq!(s.account_id(), 8983981);
    assert_eq!(s.instance(), Instance::Desktop);
    assert_eq!(s.account_type(), AccountType::Individual);
    assert_eq!(s.universe(), Universe::Public);

    let s = SteamID::from_steam2("STEAM_1:1:4491990").unwrap();
    assert_eq!(s.account_id(), 8983981);
    assert_eq!(s.instance(), Instance::Desktop);
    assert_eq!(s.account_type(), AccountType::Individual);
    assert_eq!(s.universe(), Universe::Public);

    assert_eq!(
        SteamID::from_steam2("STEAM_bogus:bogus:bogus"),
        Err(SteamIDParseError::default())
    );
}

#[test]
fn test_steam3_symmetric() {
    let steam3ids = vec![
        "[U:1:123]",
        "[U:1:123:2]",
        "[G:1:626]",
        "[A:2:165:1]",
        "[T:1:123]",
        "[c:1:123]",
        "[L:1:123]",
    ];

    for id in steam3ids {
        assert_eq!(SteamID::from_steam3(id).unwrap().steam3(), id);
    }
}

#[test]
fn test_from_steam3() {
    let s = SteamID::from_steam3("[U:1:123]").unwrap();
    assert_eq!(s.account_id(), 123);
    assert_eq!(s.instance(), Instance::Desktop);
    assert_eq!(s.account_type(), AccountType::Individual);
    assert_eq!(s.universe(), Universe::Public);

    let s = SteamID::from_steam3("[A:1:123:4]").unwrap();
    assert_eq!(s.account_id(), 123);
    assert_eq!(s.instance(), Instance::Web);
    assert_eq!(s.account_type(), AccountType::AnonGameServer);
    assert_eq!(s.universe(), Universe::Public);

    let s = SteamID::from_steam3("[L:2:123]").unwrap();
    assert_eq!(s.account_id(), 123);
    assert_eq!(s.instance(), Instance::FlagLobby);
    assert_eq!(s.account_type(), AccountType::Chat);
    assert_eq!(s.universe(), Universe::Beta);

    let s = SteamID::from_steam3("[c:3:123]").unwrap();
    assert_eq!(s.account_id(), 123);
    assert_eq!(s.instance(), Instance::FlagClan);
    assert_eq!(s.account_type(), AccountType::Chat);
    assert_eq!(s.universe(), Universe::Internal);

    assert_eq!(
        SteamID::from_steam3("[bogus:bogus:bogus]"),
        Err(SteamIDParseError::default())
    );
}

#[test]
fn test_debug_print() {
    let s = SteamID::from(157626004137848889);
    let debug = format!("{:?}", s);
    assert_eq!(
        debug,
        "SteamID(157626004137848889) {ID: 12345, Instance: Web, Type: GameServer, Universe: Beta}"
    );
}

#[test]
fn steam2_overflowing_account_id() {
    let _ = SteamID::from_steam2("STEAM_0:0:9999999999");
}
