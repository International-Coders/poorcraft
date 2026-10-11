//! Steam integration layer. The `steam` cargo feature gates the full
//! Steamworks binding; without it (or when the Steam client isn't running)
//! callers transparently use the UDP transport instead.
//!
//! Development testing uses Valve's AppID 480 (Spacewar) via the
//! steam_appid.txt file at the repo/game root — see docs/STEAM.md.

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Transport {
    Udp,
    #[cfg(feature = "steam")]
    SteamP2p,
}

impl Default for Transport {
    fn default() -> Self {
        Transport::Udp
    }
}

/// The test AppID every Steamworks dev uses before owning an app id.
pub const SPACEWAR_APPID: u32 = 480;

/// Which transport the game should use. With the `steam` feature disabled
/// (the default) this is always Udp; enabled, it reports Steam only after a
/// successful runtime init (which requires the Steam client to be running).
pub fn preferred_transport() -> Transport {
    #[cfg(feature = "steam")]
    {
        if steamworks_init_ok() {
            return Transport::SteamP2p;
        }
    }
    Transport::Udp
}

#[cfg(feature = "steam")]
fn steamworks_init_ok() -> bool {
    // steamworks::Client::init() fails when the Steam client is absent;
    // we treat that as "fall back to UDP" rather than an error.
    steamworks::Client::init().is_ok()
}

pub mod lobbies;
#[cfg(feature = "steam")]
pub mod net_steam;
pub mod workshop;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spacewar_appid_is_480() {
        assert_eq!(SPACEWAR_APPID, 480);
    }

    #[test]
    fn default_transport_is_udp() {
        // Without the binding compiled in, UDP is the only answer. With it,
        // the result depends on whether the Steam client is running — both
        // are valid, so only assert the fallback for the feature-off build.
        #[cfg(not(feature = "steam"))]
        assert_eq!(preferred_transport(), Transport::Udp);
        #[cfg(feature = "steam")]
        assert!(matches!(preferred_transport(), Transport::Udp | Transport::SteamP2p));
    }
}
