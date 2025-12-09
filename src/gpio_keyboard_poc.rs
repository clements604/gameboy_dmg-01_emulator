use rppal::gpio::{Gpio, Trigger, Event};
use std::time::Duration;
use enigo::{Enigo, KeyboardControllable};

fn main() -> rppal::gpio::Result<()> {
    let gpio = Gpio::new()?;
    let mut pin = gpio.get(26)?.into_input_pullup();

    let debounce = Some(Duration::from_millis(10));
    let enigo = std::sync::Arc::new(std::sync::Mutex::new(Enigo::new()));
    let enigo_clone = enigo.clone();

    let _interrupt = pin.set_async_interrupt(
        Trigger::Both,   // this is allowed in 0.22.x
        debounce,
        move |event: Event| {
            match event.trigger {
                Trigger::RisingEdge => {
                    println!("Button released (HIGH) at {:?}", event.timestamp);
                }
                Trigger::FallingEdge => {
                    println!("Button pressed (LOW) at {:?}", event.timestamp);
                    let mut enigo = enigo_clone.lock().unwrap();
                    enigo.key_click(enigo::Key::Layout('w'));
                }
                _ => {}
            }
        }
    )?;

    loop {
        std::thread::sleep(Duration::from_secs(1));
    }
}
