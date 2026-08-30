//! Client side: `cargo run --release -p lf_steam --features steam
//! --example steam_client -- <lobby_id>` — joins the lobby, reads the
//! host identity from its datum, connects over Steam P2P, sends the
//! protocol-v4 Hello and waits for the Welcome.

#[cfg(feature = "steam")]
fn main() {
    use lf_protocol::{ClientMessage, ProtocolCodec, PROTOCOL_VERSION};
    use lf_steam::net_steam::{SteamClientNet, SteamClientEvent};

    let args: Vec<String> = std::env::args().collect();
    let mut net = if let Some(pos) = args.iter().position(|a| a == "--host") {
        let host: u64 = args.get(pos + 1).expect("--host <steamid>").parse().expect("numeric");
        let n = SteamClientNet::connect_direct(host).expect("connect start");
        println!("DIRECT PASS: host_id={}", n.host_id);
        n
    } else {
        let lobby: u64 = args
            .get(1)
            .expect("usage: steam_client <lobby_id> | --host <steamid>")
            .parse()
            .expect("lobby id must be numeric");
        let n = SteamClientNet::join(lobby, "client").expect("join lobby + connect start");
        println!("JOIN   PASS: lobby={lobby} host_id={}", n.host_id);
        n
    };

    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(60);
    let mut hello_sent = false;
    while std::time::Instant::now() < deadline {
        for event in net.pump() {
            match event {
                SteamClientEvent::Connected => {
                    println!("CONN   PASS: P2P session up");
                    net.send(&ClientMessage::Hello {
                        name: "probe-client".into(),
                        protocol_version: PROTOCOL_VERSION,
                    });
                    hello_sent = true;
                    println!("HELLO  SENT (protocol v{})", PROTOCOL_VERSION);
                }
                SteamClientEvent::Message(sm) => {
                    if let lf_protocol::ServerMessage::Welcome { your_id, seed, players } = sm {
                        println!(
                            "EXCHANGE PASS: Welcome {{ your_id: {your_id}, seed: {seed}, players: {:?} }}",
                            players.len()
                        );
                        net.leave_lobby();
                        println!("CLIENT PASS: protocol-v4 message exchanged over Steam P2P");
                        return;
                    }
                }
                SteamClientEvent::Disconnected => {
                    println!("CLIENT FAIL: connection dropped");
                    std::process::exit(1);
                }
            }
        }
        let _ = hello_sent;
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
    println!("CLIENT FAIL: no Welcome within 60s (hello_sent={hello_sent})");
    std::process::exit(1);
}

#[cfg(not(feature = "steam"))]
fn main() {
    eprintln!("build with --features lf_steam/steam");
    std::process::exit(1);
}
