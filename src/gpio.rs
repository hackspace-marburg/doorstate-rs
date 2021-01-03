use crate::new_door_state;
use rust_pigpio::constants::{GpioMode, Pud};
use rust_pigpio::{initialize, read, set_mode, set_pull_up_down, GpioResult};
use std::{thread, time};

/// Handles button changes
/// polls the switch and does not return unless on error.
pub fn switch_handling(pin: u32, mqtt_client: &mut rumqttc::Client) -> GpioResult {
    initialize()?;
    set_pull_up_down(pin, Pud::UP)?;
    set_mode(pin, GpioMode::INPUT)?;
    // Always publish the initial state at startup
    let mut last = read(pin)?;
    new_door_state(last == 0, mqtt_client);

    loop {
        thread::sleep(time::Duration::from_secs(5));
        match read(pin) {
            Ok(new) => {
                if new != last {
                    new_door_state(new == 0, mqtt_client);
                    last = new;
                }
            }
            Err(error) => println!("Error on reading switch pin: {}", error),
        };
    }
}
