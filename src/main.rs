#[macro_use]
extern crate clap;

use clap::App;
use rumqttc::{Client, Incoming, MqttOptions, QoS};
use rust_pigpio::constants::{GpioMode, Pud};
use rust_pigpio::{read, set_mode, set_pull_up_down, GpioResult};

use std::path::Path;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::SystemTime;

mod hsmr_spaceapi;
use hsmr_spaceapi::Tuerstatus;

fn main() {
    // Command line arguments
    // TODO: Additional parameters. E.g.: broker host, broker port, mqtt keep alive, mqtt topic, verbose switch, connected pin
    let yaml = load_yaml!("cli.yml");
    let matches = App::from_yaml(yaml).get_matches();

    let wikipath = matches.value_of("wikipath").unwrap_or("/mnt/wega/");
    println!("Using pmwiki path: {}", wikipath);

    // Connect the big red switch to BCM 17/pin 11
    let pin = 17;

    // Setup MQTT client
    let mut mqttoptions = MqttOptions::new("rust-tuer", "b2s.hsmr.cc", 1883);
    mqttoptions.set_keep_alive(5);
    let (mut client, connection) = Client::new(mqttoptions, 10);
    client.subscribe("door", QoS::ExactlyOnce).unwrap();

    // Setup Mutex for communication between MQTT handler and event updater
    let current_state = Arc::new(Mutex::new(Tuerstatus {
        door_open: false,
        timestamp: unixtime_now(),
        flti_only: None,
    }));

    // Spawn Threads
    spawn_mqtt_handler(connection, String::from(wikipath), &current_state);

    spawn_event_updater(String::from(wikipath), &current_state);

    // Poll for button state and change state accordingly
    // State is only changed via MQTT as the handlers above deal with all the rest
    switch_handling(pin, &mut client).expect("Issues during GPIO Handling");
}

/// Thread handling incoming Tuerstatus mqtt messages and updating spaceapi and website
fn spawn_mqtt_handler(
    mut connection: rumqttc::Connection,
    wikistr: String,
    current_state: &Arc<Mutex<Tuerstatus>>,
) -> thread::JoinHandle<()> {
    let current_state = current_state.clone();
    thread::spawn(move || {
        let wikipath = Path::new(wikistr.as_str());
        for notification in connection.iter() {
            if let rumqttc::Event::Incoming(Incoming::Publish(inc)) = notification.unwrap() {
                let message_str = String::from_utf8(inc.payload.to_vec()).unwrap();
                if let Ok(result) = serde_json::from_str::<Tuerstatus>(message_str.as_str()) {
                    println!("Tuerstatus: {} at {}", result.door_open, result.timestamp);
                    // SpaceAPI updaten
                    hsmr_spaceapi::write_spaceapi(&wikipath, &result);
                    // Webseite updaten
                    hsmr_spaceapi::write_sitenav(&wikipath, &result);
                    // Update state mutex so regular event updates still have correct state.
                    let mut state = current_state.lock().unwrap();
                    *state = result;
                }
            }
        }
    })
}

/// Thread checking for new events and updating spaceapi accordingly
fn spawn_event_updater(
    wikistr: String,
    current_state: &Arc<Mutex<Tuerstatus>>,
) -> thread::JoinHandle<()> {
    let current_state = current_state.clone();
    thread::spawn(move || {
        let wikipath = Path::new(wikistr.as_str());
        loop {
            thread::sleep(std::time::Duration::from_secs(5 * 60));
            let state = current_state.lock().unwrap();
            hsmr_spaceapi::write_spaceapi(wikipath, &*state);
        }
    })
}

/// Handles button changes
/// uses polling in a eternal loop.
fn switch_handling(pin: u32, mqtt_client: &mut rumqttc::Client) -> GpioResult {
    set_pull_up_down(pin, Pud::UP)?;
    set_mode(pin, GpioMode::INPUT)?;
    // Always publish the initial state at startup
    let mut last = read(pin)?;
    new_door_state(last == 0, mqtt_client);

    loop {
        thread::sleep(std::time::Duration::from_secs(5));
        if let Ok(new) = read(pin) {
            if new != last {
                new_door_state(new == 0, mqtt_client);
                last = new;
            }
        };
    }
}

/// Shortcut to get current system unixtime as u64
fn unixtime_now() -> u64 {
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

/// Change current hackspace state by sending new state with current time via MQTT
fn new_door_state(state: bool, mqtt_client: &mut rumqttc::Client) {
    let new = Tuerstatus {
        door_open: state,
        flti_only: Some(false),
        timestamp: unixtime_now(),
    };

    mqtt_client
        .publish(
            "door",
            QoS::ExactlyOnce,
            true,
            serde_json::to_string(&new).unwrap(),
        )
        .unwrap();
}
