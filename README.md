# steamid-ng [![crates.io](https://img.shields.io/crates/v/steamid-ng.svg)](https://crates.io/crates/steamid-ng) [![MIT License](https://img.shields.io/crates/l/steamid-ng.svg)](https://github.com/Majora320/steamid-ng/blob/master/LICENSE) [![Docs.rs](https://docs.rs/steamid-ng/badge.svg)](https://docs.rs/steamid-ng)
An easy-to-use SteamID type with functions to parse and render steam2 and steam3 IDs. 
This library is based off of [SteamID.php](https://github.com/xPaw/SteamID.php) by xPaw.

## 1.0 to 2.0 migration notes
The primary semantic breaking change between 1.0 (and earlier version) and 2.0 is that in 1.0, a constructed `SteamID` type is not necessarily valid, while in 2.0 it is always valid. This means that the library will explicitly validate your types while parsing, and functions on it will always return valid results.
