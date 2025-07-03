use std::io;
use std::net::{SocketAddr, UdpSocket};
use std::thread::sleep;
use std::time::Duration;

fn main() {
    let addr = "[fe80::a401:1bff:fea2:3f5%10]:50349".parse::<SocketAddr>().unwrap();
    let socket = UdpSocket::bind("[::]:50349").unwrap();
    /*let mut buf = [0; 2048];
    let res = socket.recv_from(&mut buf);
    println!("res{:?}   {:?}", res, buf);*/

    let mut buf_out:Vec<u16> = Vec::with_capacity(1500);
    let mut seq = 0u32;
    let out_a_p: [u16; 1024] = [
        512, 512, 512, 512, 512, 512, 512, 512, 537, 537, 537, 537, 537, 537, 537, 537, 562, 562, 562,
        562, 562, 562, 562, 562, 587, 587, 587, 587, 587, 587, 587, 587, 612, 612, 612, 612, 612, 612,
        612, 612, 636, 636, 636, 636, 636, 636, 636, 636, 661, 661, 661, 661, 661, 661, 661, 661, 684,
        684, 684, 684, 684, 684, 684, 684, 708, 708, 708, 708, 708, 708, 708, 708, 731, 731, 731, 731,
        731, 731, 731, 731, 753, 753, 753, 753, 753, 753, 753, 753, 775, 775, 775, 775, 775, 775, 775,
        775, 796, 796, 796, 796, 796, 796, 796, 796, 817, 817, 817, 817, 817, 817, 817, 817, 837, 837,
        837, 837, 837, 837, 837, 837, 856, 856, 856, 856, 856, 856, 856, 856, 874, 874, 874, 874, 874,
        874, 874, 874, 891, 891, 891, 891, 891, 891, 891, 891, 908, 908, 908, 908, 908, 908, 908, 908,
        923, 923, 923, 923, 923, 923, 923, 923, 938, 938, 938, 938, 938, 938, 938, 938, 951, 951, 951,
        951, 951, 951, 951, 951, 964, 964, 964, 964, 964, 964, 964, 964, 975, 975, 975, 975, 975, 975,
        975, 975, 985, 985, 985, 985, 985, 985, 985, 985, 994, 994, 994, 994, 994, 994, 994, 994, 1002,
        1002, 1002, 1002, 1002, 1002, 1002, 1002, 1009, 1009, 1009, 1009, 1009, 1009, 1009, 1009, 1014,
        1014, 1014, 1014, 1014, 1014, 1014, 1014, 1018, 1018, 1018, 1018, 1018, 1018, 1018, 1018, 1022,
        1022, 1022, 1022, 1022, 1022, 1022, 1022, 1023, 1023, 1023, 1023, 1023, 1023, 1023, 1023, 1024,
        1024, 1024, 1024, 1024, 1024, 1024, 1024, 1023, 1023, 1023, 1023, 1023, 1023, 1023, 1023, 1022,
        1022, 1022, 1022, 1022, 1022, 1022, 1022, 1018, 1018, 1018, 1018, 1018, 1018, 1018, 1018, 1014,
        1014, 1014, 1014, 1014, 1014, 1014, 1014, 1009, 1009, 1009, 1009, 1009, 1009, 1009, 1009, 1002,
        1002, 1002, 1002, 1002, 1002, 1002, 1002, 994, 994, 994, 994, 994, 994, 994, 994, 985, 985,
        985, 985, 985, 985, 985, 985, 975, 975, 975, 975, 975, 975, 975, 975, 964, 964, 964, 964, 964,
        964, 964, 964, 951, 951, 951, 951, 951, 951, 951, 951, 938, 938, 938, 938, 938, 938, 938, 938,
        923, 923, 923, 923, 923, 923, 923, 923, 908, 908, 908, 908, 908, 908, 908, 908, 891, 891, 891,
        891, 891, 891, 891, 891, 874, 874, 874, 874, 874, 874, 874, 874, 856, 856, 856, 856, 856, 856,
        856, 856, 837, 837, 837, 837, 837, 837, 837, 837, 817, 817, 817, 817, 817, 817, 817, 817, 796,
        796, 796, 796, 796, 796, 796, 796, 775, 775, 775, 775, 775, 775, 775, 775, 753, 753, 753, 753,
        753, 753, 753, 753, 731, 731, 731, 731, 731, 731, 731, 731, 708, 708, 708, 708, 708, 708, 708,
        708, 684, 684, 684, 684, 684, 684, 684, 684, 661, 661, 661, 661, 661, 661, 661, 661, 636, 636,
        636, 636, 636, 636, 636, 636, 612, 612, 612, 612, 612, 612, 612, 612, 587, 587, 587, 587, 587,
        587, 587, 587, 562, 562, 562, 562, 562, 562, 562, 562, 537, 537, 537, 537, 537, 537, 537, 537,
        512, 512, 512, 512, 512, 512, 512, 512, 487, 487, 487, 487, 487, 487, 487, 487, 462, 462, 462,
        462, 462, 462, 462, 462, 437, 437, 437, 437, 437, 437, 437, 437, 412, 412, 412, 412, 412, 412,
        412, 412, 388, 388, 388, 388, 388, 388, 388, 388, 363, 363, 363, 363, 363, 363, 363, 363, 340,
        340, 340, 340, 340, 340, 340, 340, 316, 316, 316, 316, 316, 316, 316, 316, 293, 293, 293, 293,
        293, 293, 293, 293, 271, 271, 271, 271, 271, 271, 271, 271, 249, 249, 249, 249, 249, 249, 249,
        249, 228, 228, 228, 228, 228, 228, 228, 228, 207, 207, 207, 207, 207, 207, 207, 207, 187, 187,
        187, 187, 187, 187, 187, 187, 168, 168, 168, 168, 168, 168, 168, 168, 150, 150, 150, 150, 150,
        150, 150, 150, 133, 133, 133, 133, 133, 133, 133, 133, 116, 116, 116, 116, 116, 116, 116, 116,
        101, 101, 101, 101, 101, 101, 101, 101, 86, 86, 86, 86, 86, 86, 86, 86, 73, 73, 73, 73, 73, 73,
        73, 73, 60, 60, 60, 60, 60, 60, 60, 60, 49, 49, 49, 49, 49, 49, 49, 49, 39, 39, 39, 39, 39, 39,
        39, 39, 30, 30, 30, 30, 30, 30, 30, 30, 22, 22, 22, 22, 22, 22, 22, 22, 15, 15, 15, 15, 15, 15,
        15, 15, 10, 10, 10, 10, 10, 10, 10, 10, 6, 6, 6, 6, 6, 6, 6, 6, 2, 2, 2, 2, 2, 2, 2, 2, 1, 1,
        1, 1, 1, 1, 1, 1, 0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 1, 1, 1, 1, 2, 2, 2, 2, 2, 2, 2, 2, 6, 6,
        6, 6, 6, 6, 6, 6, 10, 10, 10, 10, 10, 10, 10, 10, 15, 15, 15, 15, 15, 15, 15, 15, 22, 22, 22,
        22, 22, 22, 22, 22, 30, 30, 30, 30, 30, 30, 30, 30, 39, 39, 39, 39, 39, 39, 39, 39, 49, 49, 49,
        49, 49, 49, 49, 49, 60, 60, 60, 60, 60, 60, 60, 60, 73, 73, 73, 73, 73, 73, 73, 73, 86, 86, 86,
        86, 86, 86, 86, 86, 101, 101, 101, 101, 101, 101, 101, 101, 116, 116, 116, 116, 116, 116, 116,
        116, 133, 133, 133, 133, 133, 133, 133, 133, 150, 150, 150, 150, 150, 150, 150, 150, 168, 168,
        168, 168, 168, 168, 168, 168, 187, 187, 187, 187, 187, 187, 187, 187, 207, 207, 207, 207, 207,
        207, 207, 207, 228, 228, 228, 228, 228, 228, 228, 228, 249, 249, 249, 249, 249, 249, 249, 249,
        271, 271, 271, 271, 271, 271, 271, 271, 293, 293, 293, 293, 293, 293, 293, 293, 316, 316, 316,
        316, 316, 316, 316, 316, 340, 340, 340, 340, 340, 340, 340, 340, 363, 363, 363, 363, 363, 363,
        363, 363, 388, 388, 388, 388, 388, 388, 388, 388, 412, 412, 412, 412, 412, 412, 412, 412, 437,
        437, 437, 437, 437, 437, 437, 437, 462, 462, 462, 462, 462, 462, 462, 462, 487, 487, 487, 487,
        487, 487, 487, 487,];
    for i in 0..1 {
        // transmission
        for out_a_p in out_a_p.chunks(64) {
            buf_out.clear();
            let mut pkg = [0u8; 1500];

            let sample_freq = 48_000u32;


            pkg[0..4].copy_from_slice(&seq.to_le_bytes());
            seq = seq.wrapping_add(1);
            pkg[4..8].copy_from_slice(&sample_freq.to_le_bytes());
            let mut pos = 8;

            for e in out_a_p.iter().map(|n| (*n)) {
                    let [a, b] = e.to_le_bytes();
                    pkg[pos]  = a;
                    pkg[pos + 1] = b;
                    pos += 2;
            }
            sleep(Duration::from_millis(1));
            let _ = socket.send_to(&pkg[..pos], addr).map_err(|e|println!("{:?}", e));
        }
    }
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
    let mut buf_out:Vec<u16> = Vec::with_capacity(1500);
    let mut last_time = get_time();
    let mut last_sampling_freq = 0u128;
    let process_callback = move |_: &jack::Client, ps: &jack::ProcessScope| -> jack::Control {
        let out_a_p = out_a.as_mut_slice(ps);
        let buf_size = out_a_p.len();
        if callback % CALC_EVERY == 0 {
            let time = get_time();
            let nanos_per_buffer = time.saturating_sub(last_time);
            last_time = time;
            let sampling_freq = 1_000_000_000_000 * buf_size as u128 * CALC_EVERY as u128 / nanos_per_buffer;
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
            if sample_freq > 80_000 { // ignoring extremes
                sample_freq = 48_000;
            }


            pkg[0..4].copy_from_slice(&seq.to_le_bytes());
            seq = seq.wrapping_add(1);
            pkg[4..8].copy_from_slice(&sample_freq.to_le_bytes());
            let mut pos = 8;

            for e in out_a_p.iter().map(|n| (n * u16::MAX as f32) as u16) {
                let [a, b] = e.to_le_bytes();
                pkg[pos]  = a;
                pkg[pos + 1] = b;
                pos += 2;
            }

            let _ = socket.send_to(&pkg[..pos], addr).map_err(|e|println!("{:?}", e));
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
        let _ = socket.send_to(&[0,0,0,0, 0,1,0,1,0,1,0,1,0,1,0,1,0,1,0,1], addr).map_err(|e|println!("{:?}", e));
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
    let res = unsafe {libc::clock_gettime(libc::CLOCK_REALTIME, &mut timespec) };
    assert_eq!(res, 0, "Could not get libc::clock_gettime(libc::CLOCK_REALTIME, /*...*/)");

    timespec.tv_nsec as u128 + timespec.tv_sec as u128 * 1_000_000_000u128
}