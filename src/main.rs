use std::io;
use std::net::{SocketAddr, UdpSocket};

fn main() {
    // "[fe80::a401:1bff:fea2:3f5%10]:50349"
    let addr = std::env::var("REMOTE_ADDR")
        .expect("REMOTE_ADDR environment variable not set")
        .parse::<SocketAddr>()
        .unwrap();
    let socket = UdpSocket::bind("[::]:50349").unwrap();
    /*let mut buf = [0; 2048];
    let res = socket.recv_from(&mut buf);
    println!("res{:?}   {:?}", res, buf);*/

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
    const CALC_EVERY: u32 = 512;
    let mut seq = 0u32;
    let mut callback = 0;
    let mut buf_out: Vec<u16> = Vec::with_capacity(1500);
    let mut last_time = get_time();
    let mut last_sampling_freq = 0u128;
    let process_callback = move |_: &jack::Client, ps: &jack::ProcessScope| -> jack::Control {
        let out_a_p = out_a.as_mut_slice(ps);
        let buf_size = out_a_p.len();
        if callback % CALC_EVERY == 0 {
            let time = get_time();
            let nanos_per_buffer = time.saturating_sub(last_time);
            last_time = time;
            let sampling_freq =
                1_000_000_000_000 * buf_size as u128 * CALC_EVERY as u128 / nanos_per_buffer;
            println!(
                "ns p buf {}/{} f: {}, delta: {}ppm, time: {}",
                nanos_per_buffer,
                buf_size,
                sampling_freq,
                1_000_000 - ((last_sampling_freq * 1_000_000) / sampling_freq) as i64,
                time
            );
            last_sampling_freq = sampling_freq;
        }
        callback = callback.wrapping_add(1);
        // transmission
        for out_a_p in out_a_p.chunks(64) {
            buf_out.clear();
            let mut pkg = [0u8; 1500];

            let mut sample_freq = (last_sampling_freq / 1_000) as u32;
            if sample_freq > 80_000 {
                // ignoring extremes
                sample_freq = 48_000;
            }

            pkg[0..4].copy_from_slice(&seq.to_le_bytes());
            seq = seq.wrapping_add(1);
            pkg[4..8].copy_from_slice(&sample_freq.to_le_bytes());
            let mut pos = 8;

            for e in out_a_p.iter().map(|n| (*n * i16::MAX as f32) as i16) {
                let [a, b] = e.to_le_bytes();
                for _ in 0..8 {
                    pkg[pos] = a;
                    pkg[pos + 1] = b;
                    pos += 2;
                }
            }

            let _ = socket
                .send_to(&pkg[..pos], addr)
                .map_err(|e| println!("{:?}", e));
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
    let socket = UdpSocket::bind("[::]:50348").unwrap();
    loop {
        io::stdin().read_line(&mut user_input).ok();
        let _ = socket
            .send_to(&[0, 0, 0, 0, 0, 0, 0, 0], addr)
            .map_err(|e| println!("{:?}", e));
    }

    // 5. Not needed as the async client will cease processing on `drop`.
    if let Err(err) = active_client.deactivate() {
        eprintln!("JACK exited with error: {err}");
    }
}

fn get_time() -> u128 {
    let mut timespec = libc::timespec {
        tv_sec: 0,
        tv_nsec: 0,
    };
    let res = unsafe { libc::clock_gettime(libc::CLOCK_REALTIME, &mut timespec) };
    assert_eq!(
        res, 0,
        "Could not get libc::clock_gettime(libc::CLOCK_REALTIME, /*...*/)"
    );

    timespec.tv_nsec as u128 + timespec.tv_sec as u128 * 1_000_000_000u128
}
