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
    println!("Connecting to {addr}");
    let socket = UdpSocket::bind("[::]:0").await.unwrap();
    /*let mut buf = [0; 2048];
    let res = socket.recv_from(&mut buf);
    println!("res{:?}   {:?}", res, buf);*/
    let (tx, mut rx) = mpsc::channel(4);

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

#[inline(always)]
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
    let (client, _status) = jack::Client::new(
        &format!(
            "Audio {}",
            std::env::var("REMOTE_ADDR").expect("REMOTE_ADDR environment variable not set")
        ),
        jack::ClientOptions::default(),
    )
    .unwrap();

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
    let mut callback = 0;
    let mut last_time = get_time();
    let mut last_sampling_freq = 0u128;
    // new
    let mut times = vec![(0, 0); 1024 * 16];
    let mut prev_time = get_time();
    let mut i = 0;

    let process_callback = move |client: &jack::Client, ps: &jack::ProcessScope| -> jack::Control {
        let (time, jack_time, measurement_delay) = (0..3)
            .map(|_| {
                let time = get_time();
                let jack_time = client.time();
                let time_after = get_time();
                let delay = (time_after as i128 - time as i128).abs();
                (time, jack_time, delay)
            })
            .min_by_key(|&(_, _, delay)| delay)
            .unwrap();

        if measurement_delay > 4000 {
            println!("took too long! {measurement_delay}ns");
        }

        //println!("{}", (time as i128 / 1000 - jack_time as i128));
        let cycle_times = ps.cycle_times().unwrap();

        let callback_late = (jack_time as i128 - cycle_times.current_usecs as i128) * 1_000;

        let ptp_start_time = time as i128 - callback_late;
        let ptp_start_time_frames = cycle_times.current_frames;

        /*println!(
            "diff between clocks {} ({:x} {:x}) frames {} correction {}",
            (time as i128 / 1000) - jack_time as i128,
            time as i128 / 1000,
            jack_time,
            cycle_times.current_frames,
            callback_late,
        );*/

        let time_frames = ps.frames_since_cycle_start();
        let len = times.len();

        times[i as usize % len] = (ptp_start_time, ptp_start_time_frames);
        if i % (len as u32) == 0 {
            println!("{:?}", times);
        }
        prev_time = time;
        i = i.wrapping_add(1);

        let callback_sample_number = ps.last_frame_time();
        let buf_size = in_ports[0].as_slice(ps).len();
        if callback % CALC_EVERY == 0 {
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
            let mut seq = callback_sample_number;
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
                seq = seq.wrapping_add((frame.len() / 8) as u32);
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
                    //println!("Error sending packet: {e}");
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
