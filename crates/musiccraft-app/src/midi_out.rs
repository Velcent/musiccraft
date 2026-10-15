//! MIDI output for MIDI Out channels: a thread drains the engine's outgoing
//! queue and sends each message to the chosen port.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::channel;
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::Duration;

use midir::{MidiOutput, MidiOutputConnection};
use musiccraft_engine::MidiOutReceiver;

/// How often the thread looks for new messages.
const POLL: Duration = Duration::from_millis(1);
/// How long to wait for the port to open.
const CONNECT_TIMEOUT: Duration = Duration::from_secs(3);

/// Names of the MIDI output ports.
pub fn port_names() -> Vec<String> {
    let Ok(output) = MidiOutput::new("MusicCraft") else {
        return Vec::new();
    };
    output
        .ports()
        .iter()
        .filter_map(|p| output.port_name(p).ok())
        .collect()
}

/// A running MIDI output. Dropping it silences the gear and closes the port.
pub struct MidiOut {
    stop: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
    /// Port name.
    pub name: String,
}

impl Drop for MidiOut {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

/// "All notes off" (controller 123) on every MIDI channel.
fn all_notes_off(connection: &mut MidiOutputConnection) {
    for channel in 0..16_u8 {
        let _ = connection.send(&[0xb0 | channel, 123, 0]);
    }
}

fn open(port: &str) -> Result<(MidiOutputConnection, String), String> {
    let output = MidiOutput::new("MusicCraft").map_err(|e| e.to_string())?;
    let ports = output.ports();
    let found = ports
        .iter()
        .find(|p| output.port_name(p).is_ok_and(|name| name == port))
        .ok_or_else(|| format!("MIDI output port `{port}` not found"))?;
    let name = output.port_name(found).unwrap_or_else(|_| port.to_owned());
    let connection = output
        .connect(found, "musiccraft-out")
        .map_err(|e| e.to_string())?;
    Ok((connection, name))
}

/// Opens `port` and starts sending what `receiver` yields. Messages queued
/// while no port was open are discarded first.
///
/// # Errors
///
/// Returns a description when the port is missing or cannot be opened.
pub fn start(receiver: Arc<Mutex<MidiOutReceiver>>, port: &str) -> Result<MidiOut, String> {
    let stop = Arc::new(AtomicBool::new(false));
    let (ready_tx, ready_rx) = channel();
    let port = port.to_owned();
    let flag = Arc::clone(&stop);
    let thread = std::thread::Builder::new()
        .name("musiccraft-midi-out".to_owned())
        .spawn(move || {
            // The connection is opened on this thread, so it never moves.
            let (mut connection, name) = match open(&port) {
                Ok(opened) => opened,
                Err(err) => {
                    let _ = ready_tx.send(Err(err));
                    return;
                }
            };
            if let Ok(mut rx) = receiver.lock() {
                while rx.pop().is_some() {}
            }
            let _ = ready_tx.send(Ok(name));
            while !flag.load(Ordering::Relaxed) {
                if let Ok(mut rx) = receiver.lock() {
                    while let Some(event) = rx.pop() {
                        let _ = connection.send(&event.bytes);
                    }
                }
                std::thread::sleep(POLL);
            }
            all_notes_off(&mut connection);
            let _ = connection.close();
        })
        .map_err(|e| e.to_string())?;
    let mut out = MidiOut {
        stop,
        thread: Some(thread),
        name: String::new(),
    };
    match ready_rx.recv_timeout(CONNECT_TIMEOUT) {
        Ok(Ok(name)) => {
            out.name = name;
            Ok(out)
        }
        Ok(Err(err)) => Err(err),
        Err(_) => Err("MIDI output did not open in time".to_owned()),
    }
}
