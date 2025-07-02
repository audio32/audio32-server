use std::io;
use std::net::{SocketAddr, UdpSocket};



fn main() {
    let addr = "[fe80::a401:1bff:fea2:3f5%10]:50349".parse::<SocketAddr>().unwrap();
    let socket = UdpSocket::bind("[::]:50349").unwrap();
    /*let mut buf = [0; 2048];
    let res = socket.recv_from(&mut buf);
    println!("res{:?}   {:?}", res, buf);
*/
    // 1. Create client
    let (client, _status) =
        jack::Client::new("rust_jack_simple", jack::ClientOptions::default()).unwrap();

    // 2. Register ports. They will be used in a callback that will be
    // called when new data is available.
    let in_a: jack::Port<jack::AudioIn> = client
        .register_port("rust_in_l", jack::AudioIn::default())
        .unwrap();
    let in_b: jack::Port<jack::AudioIn> = client
        .register_port("rust_in_r", jack::AudioIn::default())
        .unwrap();
    let mut out_a: jack::Port<jack::AudioOut> = client
        .register_port("rust_out_l", jack::AudioOut::default())
        .unwrap();
    let mut out_b: jack::Port<jack::AudioOut> = client
        .register_port("rust_out_r", jack::AudioOut::default())
        .unwrap();
    let process_callback = move |_: &jack::Client, ps: &jack::ProcessScope| -> jack::Control {
        let out_a_p = out_a.as_mut_slice(ps);
        for out_a_p in out_a_p.chunks(64) {
            let mut buf_out:Vec<u16> = Vec::with_capacity(1500);
            for e in out_a_p.iter().map(|n| (n * u16::MAX as f32) as u16) {
                for _ in 0..8 {
                    buf_out.push(e);
                }
            }
            let (prefix, result, suffix) = unsafe { buf_out.align_to::<u8>() };
            assert!(prefix.is_empty() && suffix.is_empty() &&
                        core::mem::align_of::<u8>() <= core::mem::align_of::<u16>(),
                    "Expected u8 alignment to be no stricter than u16 alignment");
            println!("len: {}", result.len());
            let _ = socket.send_to(result, addr).map_err(|e|println!("{:?}", e));
        }
        let out_b_p = out_b.as_mut_slice(ps);
        let in_a_p = in_a.as_slice(ps);
        let in_b_p = in_b.as_slice(ps);
        out_a_p.clone_from_slice(in_a_p);
        out_b_p.clone_from_slice(in_b_p);
        jack::Control::Continue
    };
    let process = jack::contrib::ClosureProcessHandler::new(process_callback);

    // 3. Activate the client, which starts the processing.
    let active_client = client.activate_async((), process).unwrap();

    // 4. Wait for user input to quit
    println!("Press enter/return to quit...");
    let mut user_input = String::new();
    io::stdin().read_line(&mut user_input).ok();

    // 5. Not needed as the async client will cease processing on `drop`.
    if let Err(err) = active_client.deactivate() {
        eprintln!("JACK exited with error: {err}");
    }
}