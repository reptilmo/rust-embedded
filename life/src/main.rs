#![no_main]
#![no_std]

use cortex_m_rt::entry;
use embedded_hal::{digital::OutputPin, delay::DelayNs};
use microbit::{board::Board, hal::Timer};
use panic_rtt_target as _;
use rtt_target::rtt_init_print;

const LED11: (usize, usize) = (1, 1);

#[entry]
fn main() -> ! {
    rtt_init_print!();

    let board = Board::take().unwrap();
    let mut timer = Timer::new(board.TIMER0);
    let (mut rows, mut cols) = board.display_pins.degrade();

    for r in &mut rows {
        r.set_high().unwrap();
    }

    for c in &mut cols {
        c.set_low().unwrap();
    }

    let mut switch_led = |(r, c): (usize, usize), set: bool| {
        if set {
            rows[r].set_low().unwrap();
            cols[c].set_high().unwrap();
        } else {
            rows[r].set_high().unwrap();
            cols[c].set_low().unwrap();
        }
    };

    loop {
        timer.delay_ms(1000);
        switch_led(LED11, true);
        timer.delay_ms(1000);
        switch_led(LED11, false);
    }
}
