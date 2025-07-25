use std::net::SocketAddr;
use tokio::net::UdpSocket;
use tokio::sync::mpsc;
use tokio::sync::mpsc::Sender;

#[tokio::main]
async fn main() {
    // "[fe80::a401:1bff:fea2:3f5%10]:50349"
    let addr = std::env::var("REMOTE_ADDR")
        .expect("REMOTE_ADDR environment variable not set")
        .parse::<SocketAddr>()
        .unwrap();
    let sai_interface = std::env::var("REMOTE_SAI")
        .ok()
        .and_then(|s| s.parse::<u16>().ok())
        .unwrap_or(0);
    println!("Connecting to {addr}, SAI interface: {sai_interface}");
    let socket = UdpSocket::bind(format!("[::]:{}", 50349 + sai_interface))
        .await
        .unwrap();
    /*let mut buf = [0; 2048];
    let res = socket.recv_from(&mut buf);
    println!("res{:?}   {:?}", res, buf);*/
    let (tx, mut rx) = mpsc::channel(32);

    {
        let tx0 = tx.clone();
        std::thread::spawn(move || jack_client(tx0));
    }

    loop {
        let (pkg, sai_iface) = rx.recv().await.unwrap();
        let mut socket_addr = SocketAddr::from(addr);
        /*if sai_iface > 3 {
            socket_addr.set_port(1234);
        }*/
        if let Err(e) = socket.send_to(&pkg[..], socket_addr).await {
            println!("Error sending UDP packet: {e}");
        }
    }
    /*// 5. Not needed as the async client will cease processing on `drop`.
    if let Err(err) = active_client.deactivate() {
        eprintln!("JACK exited with error: {err}");
    }*/
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

fn jack_client(sender: Sender<(Vec<u8>, u32)>) {
    // 1. Create client
    let (client, _status) =
        jack::Client::new("rust_jack_simple", jack::ClientOptions::default()).unwrap();

    // 2. Register ports. They will be used in a callback that will be
    // called when new data is available.
    let mut in_ports = Vec::new();
    for i in 0..32 {
        let a: jack::Port<jack::AudioIn> = client
            .register_port(&format!("rust_in_{i}_l",), jack::AudioIn::default())
            .unwrap();
        in_ports.push(a);
    }
    const CALC_EVERY: u32 = 512;
    let mut seq = [0u32; 4];
    let mut callback = 0;
    let mut last_time = get_time();
    let mut last_sampling_freq = 0u128;
    let process_callback = move |_: &jack::Client, ps: &jack::ProcessScope| -> jack::Control {
        let buf_size = in_ports[0].as_slice(ps).len();
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

        let slices = in_ports.iter().map(|a| a.as_slice(ps)).collect::<Vec<_>>();
        for (sai_interface, slices) in slices.chunks(8).enumerate() {
            let seq = &mut seq[sai_interface];
            let mut interleaved = Vec::with_capacity(slices.len() * slices[0].len());
            for i in 0..slices[0].len() {
                for slice in slices.iter() {
                    interleaved.push(slice[i]);
                }
            }
            callback = callback.wrapping_add(1);
            // transmission
            for frame in interleaved.chunks(90 * 8) {
                let mut pkg = [0u8; 1500];

                let mut sample_freq = (last_sampling_freq / 1_000) as u32;
                if sample_freq > 80_000 {
                    // ignoring extremes
                    sample_freq = 48_000;
                }
                let sai_interface = sai_interface as u32;
                pkg[0..4].copy_from_slice(&seq.to_le_bytes());
                *seq = seq.wrapping_add(1);
                pkg[4..8].copy_from_slice(&sai_interface.to_le_bytes());
                let mut pos = 8;
                for sample in frame.iter() {
                    let sample = (*sample * i16::MAX as f32) as i16;
                    let [a, b] = sample.to_le_bytes();
                    pkg[pos] = a;
                    pkg[pos + 1] = b;
                    pos += 2;
                }

                if let Err(e) = sender.try_send((pkg[..pos].to_vec(), sai_interface)) {
                    println!("Error sending packet: {e}");
                }
            }
        }

        jack::Control::Continue
    };
    let process = jack::contrib::ClosureProcessHandler::new(process_callback);

    // 3. Activate the client, which starts the processing.
    let active_client = client.activate_async((), process).unwrap();
    loop {
        std::thread::sleep(std::time::Duration::from_millis(1000));
    }
}
