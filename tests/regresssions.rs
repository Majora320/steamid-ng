use steamid_ng::SteamID;

#[test]
fn steam2_overflowing_account_id() {
    let _ = SteamID::from_steam2("STEAM_0:0:9999999999");
}
