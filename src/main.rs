use std::io::{Read, Write};

fn main() {
    let usage = r#"Simple wake-on-lan magic packet sender

Environment variables:
    <WAKE_MAC>          MAC of the device you want to wol.
    [WAKE_SERVER_ADDR]  Address where the server will be listening.  Default = OS provided
    [WAKE_SERVER_PORT]  Port where the server will be listening.     Default = OS provided"#;
    if std::env::args().any(|a| a == "--help" || a == "-h") {
        println!("{usage}");
        std::process::exit(0);
    }

    let mac = std::env::var("WAKE_MAC").expect("WAKE_MAC");
    let port = std::env::var("WAKE_SERVER_PORT").unwrap_or("0".into());
    let addr = std::env::var("WAKE_SERVER_ADDR").unwrap_or("0.0.0.0".into()) + ":" + &port;
    let packet = {
        let mut v = vec![0xffu8; 6];
        let mac = mac.split(':').fold(0, |mac, nums| {
            (mac << 16) | u16::from_str_radix(nums, 16).unwrap() as u64
        });
        if dbg!(mac.leading_zeros()) != 16 {
            println!("{mac:x}");
            panic!("Expected mac with format aaaa:aaaa:aaaa, found {mac:?}");
        }
        v.reserve(48 * 16);
        for n in std::iter::repeat_n(&mac.to_be_bytes()[2..], 16) {
            v.extend(n);
        }
        v
    };

    for n in &packet {
        print!("{n:x}");
    }
    println!();

    let mut udp = std::net::UdpSocket::bind("0.0.0.0:0").unwrap();
    udp.set_broadcast(true).unwrap();

    let listener = std::net::TcpListener::bind(addr).unwrap();
    println!("Listening on http://{}", listener.local_addr().unwrap());
    for client in listener.incoming().flatten() {
        dbg!(&client);
        if let Err(e) = handle_client(client, &mut udp, &packet) {
            eprintln!("{e}");
        }
    }
}

fn handle_client(
    mut client: std::net::TcpStream,
    udp: &mut std::net::UdpSocket,
    packet: &[u8],
) -> std::io::Result<()> {
    client.read_exact(&mut [0, 0, 0, 0])?; // "GET "

    let mut route = [0, 0];
    client.read_exact(&mut route)?; // "/[w]"

    match &route {
        b"/w" => handle_wake(udp, packet)?,
        _ => return Err(std::io::Error::last_os_error()),
    }

    let response = "HTTP/1.1 200 OK\r\n\r\n";
    client.write_all(response.as_bytes())?;

    eprintln!("Closing connection with client");
    Ok(())
}

fn handle_wake(udp: &mut std::net::UdpSocket, packet: &[u8]) -> std::io::Result<()> {
    eprintln!("Sending...");
    udp.send_to(
        packet,
        std::net::SocketAddrV4::new(std::net::Ipv4Addr::BROADCAST, 9),
    )
    .unwrap();

    Ok(())
}