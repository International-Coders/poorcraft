//! Steam end-to-end probe (run from the repo root so steam_appid.txt is
//! visible): `cargo run -p lf_steam --features steam --example steam_probe`
//!
//! Exercises, in order: Steamworks init (needs the Steam client running
//! with a logged-in user), identity, matchmaking lobby create/leave,
//! user stats/achievements request, and overlay availability. Prints one
//! PASS/FAIL line per step and exits 0 only if INIT passed.

#[cfg(feature = "steam")]
fn main() {
    use steamworks::LobbyType;

    println!("probe: transport = {:?}", lf_steam::preferred_transport());

    let client = match steamworks::Client::init() {
        Ok(c) => c,
        Err(e) => {
            println!("INIT   FAIL: {e} (Steam client running and logged in?)");
            std::process::exit(1);
        }
    };
    println!("INIT   PASS");

    let me = client.user().steam_id();
    println!("ID     PASS: steam_id={} account={}", me.raw(), me.account_id().raw());

    // stats + achievements for the current appid (Spacewar 480 in dev)
    client.user_stats().request_user_stats(me.raw());
    println!("STATS  PASS (requested for {})", me.raw());

    // matchmaking: create a public friends-only lobby, read it back, leave
    let (tx, rx) = std::sync::mpsc::channel();
    client.matchmaking().create_lobby(LobbyType::Public, 8, move |r| {
        let _ = tx.send(r);
    });
    // matchmaking results arrive through the callback pump: pump while
    // waiting, or the callback never runs
    let mut created = Err(std::sync::mpsc::RecvTimeoutError::Timeout);
    for _ in 0..100 {
        client.run_callbacks();
        if let Ok(r) = rx.try_recv() {
            created = Ok(r);
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
    match created {
        Ok(Ok(id)) => {
            println!("LOBBY  PASS: id={}", id.raw());
            client.matchmaking().leave_lobby(id);
            println!("LEAVE  PASS");
        }
        other => println!("LOBBY  FAIL: {other:?}"),
    }

    // overlay: reports whether the in-game overlay is available for this
    // launch (it is only usable when the game runs through the Steam
    // client; direct launches report disabled)
    let overlay = client.utils().is_overlay_enabled();
    println!("OVERLAY {}", if overlay { "PASS (enabled)" } else { "WARN (disabled — launch through the Steam client)" });

    // pump callbacks a little before exiting
    for _ in 0..30 {
        client.run_callbacks();
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
    println!("probe: DONE");
}

#[cfg(not(feature = "steam"))]
fn main() {
    eprintln!("probe: build with --features lf_steam/steam");
    std::process::exit(1);
}
