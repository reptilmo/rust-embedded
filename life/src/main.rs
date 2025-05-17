// main.rs
// Kostya Leshenko
// CS410(Rust Embedded)
// Homework 1
#![no_main]
#![no_std]

use cortex_m_rt::entry;
use embedded_hal::{delay::DelayNs, digital::InputPin, digital::OutputPin};
use microbit::{
    board::Board, display::blocking::Display, hal::Timer, hal::gpio::Level, hal::rng::Rng as HalRng,
};
use nanorand::{Rng, pcg64::Pcg64};
use panic_rtt_target as _;
use rtt_target::{rprintln, rtt_init_print};

mod life;
use life::*;

#[allow(clippy::needless_range_loop)]
fn randomize(leds: &mut [[u8; 5]; 5], hal_rng: &mut HalRng) {
    let mut seed = [0u8; 16];
    hal_rng.random(&mut seed);
    let mut rng = Pcg64::new_seed(u128::from_ne_bytes(seed));
    for i in 0..5 {
        for j in 0..5 {
            leds[i][j] = rng.generate_range(0_u8..=1);
        }
    }
}

fn make_sound<D: DelayNs, Speaker: OutputPin>(delay: &mut D, speaker: &mut Speaker, val: u16) {
    for _ in 0u16..val {
        speaker.set_high().unwrap();
        delay.delay_us(500);
        speaker.set_low().unwrap();
        delay.delay_us(500);
    }
}

#[allow(clippy::needless_range_loop)]
fn complement(leds: &mut [[u8; 5]; 5]) {
    for i in 0..5 {
        for j in 0..5 {
            leds[i][j] ^= 0b0000_0001;
        }
    }
}

#[entry]
fn main() -> ! {
    rtt_init_print!();
    let board = Board::take().unwrap();
    let mut timer = Timer::new(board.TIMER0);
    let mut display = Display::new(board.display_pins);
    let mut rng = HalRng::new(board.RNG);
    let mut button_a = board.buttons.button_a;
    let mut button_b = board.buttons.button_b;
    let mut speaker = board
        .speaker_pin
        .into_push_pull_output(Level::Low);
        //.degrade();

    let mut leds = [[0u8; 5]; 5];
    randomize(&mut leds, &mut rng);

    let mut counter_curr = 0u64;
    let mut counter_prev = 0u64;
    let mut random_counter = 0u32;

    loop {
        let pressed_a = button_a.is_low().unwrap();
        let pressed_b = button_b.is_low().unwrap();
        if counter_curr - counter_prev >= 5u64 && pressed_b {
            complement(&mut leds);
            counter_prev = counter_curr;
        } else if pressed_a {
            if random_counter >= 20 {
                game(
                    &mut display,
                    &mut timer,
                    &mut button_a,
                    &mut button_b,
                    &mut rng,
                    &mut speaker,
                );
                random_counter = 0;
            }
            randomize(&mut leds, &mut rng);
            random_counter += 1;
        } else {
            life(&mut leds);
        }

        if done(&leds) {
            display.show(&mut timer, leds, 500);
            counter_curr += 5u64;
        } else {
            display.show(&mut timer, leds, 100);
            counter_curr += 1u64;
        }
    }
}

#[allow(clippy::needless_range_loop)]
fn clear(buf: &mut [[u8; 5]; 5]) {
    for i in 0..5 {
        for j in 0..5 {
            buf[i][j] = 0;
        }
    }
}

fn game<T: DelayNs, ButtonA: InputPin, ButtonB: InputPin, Speaker: OutputPin>(
    display: &mut Display,
    timer: &mut T,
    button_a: &mut ButtonA,
    button_b: &mut ButtonB,
    hal_rng: &mut HalRng,
    speaker: &mut Speaker,
) {
    let mut buf = [[0u8; 5]; 5];
    let mut lx = 2i16;
    let mut rx = 3i16;
    let mut new_bomb = true;
    let mut bomb_x = 0i16;
    let mut bomb_y = 0i16;
    let mut score = 0i16;

    let mut seed = [0u8; 16];
    hal_rng.random(&mut seed);
    let mut rng = Pcg64::new_seed(u128::from_ne_bytes(seed));

    loop {
        if score >= 10 {
            break;
        }

        if new_bomb {
            bomb_x = rng.generate_range(0_u8..=4) as i16; // There is a bug in nanorand it seems.
            bomb_y = 0;
            new_bomb = false;
        } else {
            bomb_y += 1;
        }

        let pressed_a = button_a.is_low().unwrap();
        let pressed_b = button_b.is_low().unwrap();

        if pressed_a {
            lx -= 1;
            rx -= 1;
        }

        if pressed_b {
            lx += 1;
            rx += 1;
        }

        if rx > 4 {
            rx = 4;
            lx = 3;
        } else if lx < 0 {
            lx = 0;
            rx = 1;
        }

        clear(&mut buf);
        buf[4][lx as usize] = 1;
        buf[4][rx as usize] = 1;

        if bomb_y < 4 {
            buf[bomb_y as usize][bomb_x as usize] = 1;
        } else if bomb_y == 4 && (bomb_x == lx || bomb_x == rx) {
            score += 1;
            rprintln!("Hit {}", score);
            make_sound(timer, speaker, 10);
            new_bomb = true;
        } else {
            rprintln!("Miss");
            new_bomb = true;
        }

        display.show(timer, buf, 100);
    }
}
