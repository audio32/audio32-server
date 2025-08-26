extern crate core;

use az::CheckedCast;
use std::io;
use std::net::SocketAddr;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::net::UdpSocket;
use tokio::sync::mpsc;
use tokio::sync::mpsc::Sender;

const INTERFACES_PER_DEVICE: usize = 4;
const CHANNELS_PER_INTERFACE: usize = 8;

// Set this to true and server will send only zeros except a 0xFFFF all N samples
// very useful for synchronization perfing
const DEBUG_SYNC: bool = false;

const MAXIMUM_BUFFER_ALLOWED: usize = 4096 * CHANNELS_PER_INTERFACE;

type Sample = i16;

struct Samples {
    samples: [[Sample; MAXIMUM_BUFFER_ALLOWED]; INTERFACES_PER_DEVICE],
    len: usize,
    frame: u32,
    time: u128,
}

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
    let debug_sync = Arc::new(AtomicBool::new(false));

    {
        let debug_sync = debug_sync.clone();
        let tx0 = tx.clone();
        std::thread::spawn(move || jack_client(tx0, debug_sync));
    }

    loop {
        let sample = rx.recv().await.unwrap();
        let mut socket_addr = SocketAddr::from(addr);
        /*if sai_iface > 3 {
            socket_addr.set_port(1234);
        }*/
        let is_first = true;
        for sai_interface in 0..INTERFACES_PER_DEVICE {
            let samples = &sample.samples[sai_interface][..sample.len];
            let mut seq = sample.frame;
            // transmission
            for frame in samples.chunks(85 * 8) {
                let mut pkg = [0u8; 1500];

                let mut flags: u32 = (sai_interface & 0b11) as u32;
                if is_first {
                    flags |= 1 << 3;
                }
                pkg[0..4].copy_from_slice(&flags.to_le_bytes());
                // writing sequence number
                pkg[4..8].copy_from_slice(&seq.to_le_bytes());
                seq = seq.wrapping_add((frame.len() / 8) as u32);

                let mut pos = 8;
                if is_first {
                    // put in timestamp
                    let size = size_of_val(&sample.time);
                    pkg[pos..pos + size].copy_from_slice(&sample.time.to_le_bytes());
                    pos += size;
                }

                for sample in frame.iter() {
                    let [a, b] = sample.to_le_bytes();
                    pkg[pos] = a;
                    pkg[pos + 1] = b;
                    pos += 2;
                }

                if let Err(e) = socket.send_to(&pkg[..pos], socket_addr).await {
                    println!("Error sending UDP packet: {e}");
                }
            }
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

fn jack_client(sender: Sender<Box<Samples>>, debug_sync: Arc<AtomicBool>) {
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
    const CALC_EVERY: u32 = 48_000;
    let mut last_time = get_time();
    let mut last_sample = 0;
    let mut last_sampling_freq = 0f64;
    let mut last_info_show = 0;
    // new
    let mut times = vec![(0, 0); 1024 * 16];
    let mut prev_time = get_time();
    let mut i = 0;
    let debug_sync2 = debug_sync.clone();
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

        let ptp_start_time = (time as i128 - callback_late);
        let ptp_start_time: u128 = ptp_start_time
            .checked_cast()
            .expect("timestamp should not be negative");
        let ptp_start_time_frames = cycle_times.current_frames;

        let len = times.len();

        times[i as usize % len] = (ptp_start_time, ptp_start_time_frames);
        if i % (len as u32) == 0 {
            //println!("{:?}", times);
        }
        prev_time = ptp_start_time as u128;
        i = i.wrapping_add(1);

        let buf_size = in_ports[0].as_slice(ps).len();
        let tmp_samples_passed = ptp_start_time_frames / 48_000 * 5;
        if tmp_samples_passed != last_info_show {
            last_info_show = tmp_samples_passed;

            let samples_passed = ptp_start_time_frames - last_sample;
            let nanos_per_buffer = ptp_start_time.saturating_sub(last_time);
            let sampling_freq = (samples_passed as f64 * 1e9) / nanos_per_buffer as f64;

            last_sample = ptp_start_time_frames;
            last_time = ptp_start_time;

            println!(
                "ns p buf {}/{} f: {:.3}Hz, delta: {:>5}ppm, time: {}",
                nanos_per_buffer,
                buf_size,
                sampling_freq,
                (1_000_000.0 - ((last_sampling_freq * 1_000_000.0) / sampling_freq)) as i64,
                time
            );
            last_sampling_freq = sampling_freq;
        }

        let slices = in_ports.iter().map(|a| a.as_slice(ps)).collect::<Vec<_>>();

        let mut samples = Box::new(Samples {
            samples: [[0; MAXIMUM_BUFFER_ALLOWED]; INTERFACES_PER_DEVICE],
            len: slices[0].len() * CHANNELS_PER_INTERFACE,
            frame: ptp_start_time_frames,
            time: ptp_start_time,
        });
        // split 32 channels by 8 for each sai interface
        for (sai_interface, slices) in slices.chunks(CHANNELS_PER_INTERFACE).enumerate() {
            let mut seq = ptp_start_time_frames;

            //let mut interleaved = Vec::with_capacity(slices.len() * slices[0].len());
            let mut pos = 0;
            assert!(
                samples.len <= MAXIMUM_BUFFER_ALLOWED,
                "Maximum buffer size allowed {MAXIMUM_BUFFER_ALLOWED} but it is {}",
                samples.len
            );
            for i in 0..slices[0].len() {
                for slice in slices.iter() {
                    let sample = slice[i];
                    // convert f32 sample into i16 sample
                    let sample = (sample * i16::MAX as f32) as i16;
                    if !debug_sync.load(Ordering::Relaxed) {
                        samples.samples[sai_interface][pos] = sample;
                    }
                    pos += 1;
                }
            }
            // set evey 32 * 64th sample to 0xFFFF to perf synchronization
            if debug_sync.load(Ordering::Relaxed) {
                if ptp_start_time_frames % (32 * 64) == 0 {
                    let tmp = [-1 as Sample; 8];
                    for interface in samples.samples.iter_mut() {
                        interface[..8].copy_from_slice(&tmp);
                    }
                }
            }
        }
        let len = samples.len;

        if let Err(e) = sender.try_send(samples) {
            println!("could not set packet to network thread: {e}");
        }

        jack::Control::Continue
    };
    let process = jack::contrib::ClosureProcessHandler::new(process_callback);

    // 3. Activate the client, which starts the processing.
    let active_client = client.activate_async((), process).unwrap();
    loop {
        let stdin = io::stdin();
        let mut buf = String::new();
        let res = stdin.read_line(&mut buf);
        let status = match !debug_sync2.fetch_xor(true, Ordering::Relaxed) {
            true => "on",
            false => "off",
        };
        println!("Synchronization debug mode {}", status);
    }
}
