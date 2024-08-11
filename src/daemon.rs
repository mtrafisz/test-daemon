use log::{info, warn, error};
use simplelog::*;
use pretty_bytes::converter::convert;
use signal_hook::{iterator::Signals, consts::{SIGINT, SIGTERM}};

use std::io::{Read, Write};
use std::net::{IpAddr, TcpListener, TcpStream};
use std::process::Command;
use std::process::Stdio;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

mod shared;
use shared::PORT;

fn get_default_interface() -> String {
    // this all is one line in C :3 popen

    let cmd_1 = Command::new("route")
        .stdout(Stdio::piped())
        .spawn()
        .expect("failed to execute route");

    let cmd_1_output = cmd_1.stdout.expect("failed to capture stdout");

    let cmd_2 = Command::new("grep")
        .arg("^default")
        .stdin(Stdio::from(cmd_1_output))
        .stdout(Stdio::piped())
        .spawn()
        .expect("failed to execute grep");

    let cmd_2_output = cmd_2.stdout.expect("failed to capture stdout");

    let cmd_3 = Command::new("grep")
        .arg("-o")
        .arg("[^ ]*$")
        .stdin(Stdio::from(cmd_2_output))
        .stdout(Stdio::piped())
        .spawn()
        .expect("failed to execute grep");

    let mut cmd_3_output = cmd_3.stdout.expect("failed to capture stdout");
    let mut result = String::new();
    cmd_3_output.read_to_string(&mut result).expect("failed to read stdout");

    result.trim().to_string()
}

struct BytePair {
    rx: u64,
    tx: u64,
}

fn get_bytes_transfered(iface: String) -> std::io::Result<BytePair> {
    let rx_bytes = std::fs::read_to_string(format!("/sys/class/net/{}/statistics/rx_bytes", iface))?;
    let tx_bytes = std::fs::read_to_string(format!("/sys/class/net/{}/statistics/tx_bytes", iface))?;

    Ok(BytePair {
        rx: rx_bytes.trim().parse().unwrap(),
        tx: tx_bytes.trim().parse().unwrap(),
    })
}

fn main() {
    CombinedLogger::init(
        vec![
            WriteLogger::new(LevelFilter::Info, Config::default(), std::fs::File::create("daemon.log").unwrap()),
        ]
    ).unwrap();
    info!("Starting daemon");

    let mut signals = Signals::new(&[SIGINT, SIGTERM]).expect("failed to create signal handler");

    let byte_diff = Arc::new(Mutex::new(BytePair { rx: 0, tx: 0 }));
    let running = Arc::new(Mutex::new(true));
    let running_updater = running.clone();
    let running_signal = running.clone();
    let running_server = running.clone();

    // update the byte_diff every second
    thread::spawn({
        let byte_diff = byte_diff.clone();
        let iface = get_default_interface();

        move || {
            let mut last = Instant::now();
            let mut last_bytes = get_bytes_transfered(iface.clone()).expect("failed to get bytes");
            loop {
                let now = Instant::now();
                let diff = now.duration_since(last);
                if diff.as_secs() >= 1 {
                    last = now;
                    let new_bytes = get_bytes_transfered(iface.clone()).expect("failed to get bytes");
                    let mut byte_diff = byte_diff.lock().unwrap();
                    let rx_diff = new_bytes.rx - last_bytes.rx;
                    let tx_diff = new_bytes.tx - last_bytes.tx;
                    
                    byte_diff.rx = rx_diff;
                    byte_diff.tx = tx_diff;
                    last_bytes = new_bytes;

                    let running = running_updater.lock().unwrap();
                    if !*running {
                        break;
                    }
                }
            }
        }
    });

    thread::spawn({
        move || {
            for sig in signals.forever() {
                info!("Received signal {:?}", sig);
                let mut running = running_signal.lock().unwrap();
                *running = false;
                break;
            }
        }
    });
    
    let listener = TcpListener::bind(format!("0.0.0.0:{}", PORT)).expect("failed to bind to port");
    thread::spawn({
        let byte_diff = byte_diff.clone();
        let running = running_server.clone();
        move || {
            for stream in listener.incoming() {
                let mut stream = stream.expect("failed to accept connection");
                let byte_diff = byte_diff.lock().unwrap();
                let pretty_rx = convert(byte_diff.rx as f64);
                let pretty_tx = convert(byte_diff.tx as f64);
                let pretty = format!("RX: {}/s, TX: {}/s", pretty_rx, pretty_tx);

                stream.write_all(pretty.as_bytes()).expect("failed to write to stream");
                stream.flush().expect("failed to flush stream");

                stream.shutdown(std::net::Shutdown::Both).expect("failed to shutdown stream");

                let running = running.lock().unwrap();
                if !*running {
                    break;
                }
            }
        }
    });

    loop {
        let running = running_server.lock().unwrap();
        if !*running {
            break;
        }
    }

}