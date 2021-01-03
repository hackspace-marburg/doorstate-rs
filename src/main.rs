#[macro_use]
extern crate clap;

use clap::App;
use rumqttc::{Client, Incoming, MqttOptions, QoS};

use std::path::Path;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::SystemTime;

mod hsmr_spaceapi;
use hsmr_spaceapi::Tuerstatus;

struct Settings {
    wikipath: String,
    mqttbroker: String,
    mqttport: u16,
    mqtttopic: String,
    switch_enabled: bool,
    #[cfg(feature = "gpio-support")]
    switch_pin: u32,
}

fn main() {
    // Read and parse command line arguments
    let settings = command_line_arguments();

    println!("Using pmwiki path: {}", settings.wikipath);

    // Setup MQTT client
    let mut mqttoptions = MqttOptions::new("doorstate-rs", settings.mqttbroker, settings.mqttport);
    mqttoptions.set_keep_alive(5);
    let (mut client, connection) = Client::new(mqttoptions, 10);
    client
        .subscribe(settings.mqtttopic, QoS::ExactlyOnce)
        .unwrap();

    // Setup Mutex for communication between MQTT handler and event updater
    let current_state = Arc::new(Mutex::new(Tuerstatus {
        door_open: false,
        timestamp: unixtime_now(),
        flti_only: None,
    }));

    // Spawn Threads
    spawn_mqtt_handler(connection, &settings.wikipath, &current_state);

    spawn_event_updater(&settings.wikipath, &current_state);

    // If necessary: start listening for switch state changes
    if settings.switch_enabled {
        // Poll for button state and change state accordingly
        // State is only changed via MQTT as the handlers above deal with all the rest
        #[cfg(feature = "gpio-support")]
        {
            mod gpio;
            gpio::switch_handling(settings.switch_pin, &mut client)
                .expect("Issues during GPIO Handling");
        }
        #[cfg(not(feature = "gpio-support"))]
        println!("Feature for raspberry pi gpio-support not enabled. Switch detection not possible.\nPlease recompile with active feature");
    } else {
        // In case no switch pin is given (i.e. this is run on a different system than the switch)
        // simply loop so the threads don't get killed
        println!("Running in no-physical-doorswitch mode");
        loop {}
    }
}

/// Handles parsing of command line arguments into the Settings struct using clap
fn command_line_arguments() -> Settings {
    // TODO: Additional cli parameters. E.g.: mqtt keep alive, pullup/down, maybe: separate Spaceapi.json location?
    let yaml = load_yaml!("cli.yml");
    let matches = App::from_yaml(yaml).get_matches();

    let switch_enabled;
    let _switch_pin;
    match matches.value_of("switch") {
        Some(pin) => {
            switch_enabled = true;
            _switch_pin = pin
                .parse::<u32>()
                .expect("switch_pin must be a positive number");
        }
        None => {
            switch_enabled = false;
            _switch_pin = 0;
        }
    }

    Settings {
        wikipath: matches.value_of("wikipath").unwrap().to_string(), // These first two parameters are set as required in cli.yml
        mqttbroker: matches.value_of("broker").unwrap().to_string(), // therefor we don't need to unwrap_or as they will be set when this is reached
        mqttport: matches
            .value_of("broker_port")
            .unwrap_or("1883")
            .parse::<u16>()
            .expect("Broker port must be a 16 bit number."),
        mqtttopic: matches.value_of("topic").unwrap_or("door").to_string(),
        switch_enabled: switch_enabled,
        #[cfg(feature = "gpio-support")]
        switch_pin: _switch_pin,
    }
}

/// Thread handling incoming Tuerstatus mqtt messages and updating spaceapi and website
fn spawn_mqtt_handler(
    mut connection: rumqttc::Connection,
    wikistr: &String,
    current_state: &Arc<Mutex<Tuerstatus>>,
) -> thread::JoinHandle<()> {
    let current_state = current_state.clone();
    let wikistr = wikistr.clone();
    thread::spawn(move || {
        let wikipath = Path::new(wikistr.as_str());
        for notification in connection.iter() {
            // The following is to only look at incoming messages (no keepalives or sent messages)
            if let rumqttc::Event::Incoming(Incoming::Publish(inc)) = notification.unwrap() {
                let message_str = String::from_utf8(inc.payload.to_vec()).unwrap();
                match serde_json::from_str::<Tuerstatus>(message_str.as_str()) {
                    Ok(result) => {
                        println!("Tuerstatus: {} at {}", result.door_open, result.timestamp);
                        // SpaceAPI update
                        hsmr_spaceapi::write_spaceapi(&wikipath, &result);
                        // Website update
                        if let Err(error) = hsmr_spaceapi::write_sitenav(&wikipath, &result) {
                            println!("Error writing {}/Site.SiteNav : {}", wikistr, error);
                        }
                        // Update state mutex so regular spaceapi event updates still have correct state.
                        let mut state = current_state.lock().unwrap();
                        *state = result;
                    }
                    Err(error) => println!("Error with parsing incoming MQTT message: {}", error),
                }
            }
        }
    })
}

/// Thread checking for new events and updating spaceapi accordingly
fn spawn_event_updater(
    wikistr: &String,
    current_state: &Arc<Mutex<Tuerstatus>>,
) -> thread::JoinHandle<()> {
    let current_state = current_state.clone();
    let wikistr = wikistr.clone();
    thread::spawn(move || {
        let wikipath = Path::new(wikistr.as_str());
        loop {
            thread::sleep(std::time::Duration::from_secs(5 * 60));
            let state = current_state.lock().unwrap();
            hsmr_spaceapi::write_spaceapi(wikipath, &*state);
        }
    })
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
