//! Local loopback exercise of the Steam transport (no second account!):
//! `cargo run --release -p lf_steam --features steam --example steam_pair_test`
//!
//! Uses Valve's `CreateSocketPair` (built for testing) to get two
//! already-connected connections in-process, then drives the exact
//! protocol-v4 exchange the game performs: Hello(4) -> Welcome.

#[cfg(feature = "steam")]
fn main() {
    use lf_protocol::{ClientMessage, ProtocolCodec, ServerMessage, PROTOCOL_VERSION};
    use lf_steam::net_steam::create_local_pair;

    // the client handle must outlive the pair (dropping it shuts the API down)
    let (mut host_end, mut client_end, _client) =
        create_local_pair().expect("socket pair (is the Steam client running?)");
    println!("PAIR   PASS: loopback connections created");

    // client -> host: protocol-v4 Hello
    let hello = ProtocolCodec::encode_client(&ClientMessage::Hello {
        name: "pair-probe".into(),
        protocol_version: PROTOCOL_VERSION,
    });
    assert!(host_end.receive_raw(16).is_empty(), "nothing queued before send");
    assert!(client_end.send_raw(&hello, true), "client send failed");
    client_end.flush();

    // host pumps and answers with a Welcome (the game's join reply)
    let mut welcomed_back = false;
    for _ in 0..50 {
        for data in host_end.receive_raw(16) {
            match ProtocolCodec::decode_client(&data) {
                Some(ClientMessage::Hello { name, protocol_version }) => {
                    println!(
                        "HELLO  PASS: {} protocol_version={}", name, protocol_version
                    );
                    assert_eq!(protocol_version, PROTOCOL_VERSION);
                    let welcome = ProtocolCodec::encode_server(&ServerMessage::Welcome {
                        your_id: 1,
                        seed: 42,
                        players: vec![(1, name)],
                    });
                    assert!(host_end.send_raw(&welcome, true), "host send failed");
                    host_end.flush();
                    welcomed_back = true;
                }
                other => println!("OTHER  {other:?}"),
            }
        }
        if welcomed_back {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
    assert!(welcomed_back, "host never answered the Hello");

    // client receives the Welcome — the full v4 round trip
    let mut got = None;
    for _ in 0..50 {
        for data in client_end.receive_raw(16) {
            got = ProtocolCodec::decode_server(&data);
            if got.is_some() {
                break;
            }
        }
        if got.is_some() {
            break;
        }
        client_end.flush();
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
    match got {
        Some(ServerMessage::Welcome { your_id, seed, players }) => {
            println!(
                "EXCHANGE PASS: Welcome {{ your_id: {your_id}, seed: {seed}, players: {} }}",
                players.len()
            );
            println!("PAIRTEST PASS: protocol-v4 round trip over Steam loopback");
        }
        other => {
            println!("EXCHANGE FAIL: {other:?}");
            std::process::exit(1);
        }
    }
}

#[cfg(not(feature = "steam"))]
fn main() {
    eprintln!("build with --features lf_steam/steam");
    std::process::exit(1);
}
