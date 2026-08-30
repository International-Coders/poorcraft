//! Host side of the Steam-transport end-to-end exercise (AppID 480):
//! `cargo run --release -p lf_steam --features steam --example steam_host`
//! Prints the lobby id to hand to `steam_client <id>`, then waits for the
//! client's protocol-v4 Hello and answers with a Welcome.

#[cfg(feature = "steam")]
fn main() {
    use lf_protocol::ServerMessage;
    use lf_steam::net_steam::{HostEvent, SteamHost};

    let mut host = lf_steam::net_steam::SteamGameServerHost::bind()
        .expect("steam gameserver init");
    println!("HOST PASS: gameserver_id={}", host.host_id);

    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(90);
    let mut exchanged = false;
    while std::time::Instant::now() < deadline && !exchanged {
        for event in host.pump() {
            match event {
                HostEvent::PeerConnected(peer) => println!("PEER   PASS: connected {peer}"),
                HostEvent::PeerMessage(peer, cm) => match cm {
                    lf_protocol::ClientMessage::Hello { name, protocol_version } => {
                        println!(
                            "HELLO  PASS: {name} protocol_version={protocol_version}"
                        );
                        assert_eq!(protocol_version, lf_protocol::PROTOCOL_VERSION);
                        host.send_to(peer, &ServerMessage::Welcome {
                            your_id: 1,
                            seed: 42,
                            players: vec![(1, name)],
                        });
                        println!("WELCOME SENT");
                        exchanged = true;
                    }
                    other => println!("OTHER  msg {other:?}"),
                },
                HostEvent::PeerDisconnected(peer) => println!("PEER   left {peer}"),
            }
        }
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
    if exchanged {
        println!("HOST   PASS: protocol-v4 message exchanged over Steam P2P");
    } else {
        println!("HOST   FAIL: no exchange within 90s");
        std::process::exit(1);
    }
}

#[cfg(not(feature = "steam"))]
fn main() {
    eprintln!("build with --features lf_steam/steam");
    std::process::exit(1);
}
