use rppal::gpio::{Gpio, Trigger, Event};
use std::time::Duration;

fn main() -> rppal::gpio::Result<()> {
    let gpio = Gpio::new()?;
    let mut pin = gpio.get(26)?.into_input_pullup();

    let debounce = Some(Duration::from_millis(10));

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
                }
                _ => {}
            }
        }
    )?;

    loop {
        std::thread::sleep(Duration::from_secs(1));
    }
}
