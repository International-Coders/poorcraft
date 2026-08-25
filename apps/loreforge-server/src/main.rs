use std::net::{UdpSocket, SocketAddr};
use lf_protocol::{ServerMessage, ProtocolCodec};

fn main() {
    println!("LOREFORGE Dedicated Server starting...");
    let addr = "127.0.0.1:25565";
    let socket = UdpSocket::bind(addr).expect("Failed to bind UDP socket");
    println!("Server listening on {}", addr);

    let mut buf = [0u8; 1024];
    loop {
        match socket.recv_from(&mut buf) {
            Ok((len, src)) => {
                if let Some(msg) = ProtocolCodec::decode(&buf[..len]) {
                    println!("Received from {}: {:?}", src, msg);
                    // Echo back handshake for testing
                    if matches!(msg, ServerMessage::Handshake { .. }) {
                        let resp = ServerMessage::Handshake { protocol_version: 1 };
                        let encoded = ProtocolCodec::encode(&resp);
                        socket.send_to(&encoded, src).ok();
                    }
                }
            }
            Err(e) => eprintln!("recv error: {}", e),
        }
    }
}