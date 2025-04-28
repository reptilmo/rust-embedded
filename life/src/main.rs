// main.rs
// Kostya Leshenko
// CS410(Rust Embedded)
// Homework 1
#![no_main]
#![no_std]

use core::sync::atomic::{
    AtomicBool,
    Ordering::{Acquire, Release},
};
use cortex_m_rt::entry;
use critical_section_lock_mut::LockMut;
use embedded_hal::digital::InputPin;
use microbit::{
    board::Board,
    display::blocking::Display,
    hal::Timer,
    hal::gpio::{Floating, Input, Pin},
    hal::gpiote::Gpiote,
    hal::pac::{self, interrupt},
    hal::rng::Rng as HalRng,
};
use nanorand::{Rng, pcg64::Pcg64};
use panic_rtt_target as _;
use rtt_target::rtt_init_print;

mod life;
use life::*;

struct AppState {
    gpiote: Gpiote,
    button_a: Pin<Input<Floating>>,
    button_b: Pin<Input<Floating>>,
}

static APP_STATE: LockMut<AppState> = LockMut::new();
static BUTTON_A_STATE: AtomicBool = AtomicBool::new(false);
static BUTTON_B_STATE: AtomicBool = AtomicBool::new(false);

#[interrupt]
fn GPIOTE() {
    APP_STATE.with_lock(|app_state| {
        let button_a_changed = app_state.gpiote.channel0().is_event_triggered();
        if button_a_changed {
            let button_value = app_state.button_a.is_low().unwrap();
            BUTTON_A_STATE.store(button_value, Release);
        }

        let button_b_changed = app_state.gpiote.channel1().is_event_triggered();
        if button_b_changed {
            let button_value = app_state.button_b.is_low().unwrap();
            BUTTON_B_STATE.store(button_value, Release);
        }

        app_state.gpiote.channel0().reset_events();
        app_state.gpiote.channel1().reset_events();
    });
}

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

fn complement(leds: &[[u8; 5]; 5]) -> [[u8; 5]; 5] {
    let mut out = [[0u8; 5]; 5];
    for i in 0..5 {
        for j in 0..5 {
            match leds[i][j] {
                0u8 => out[i][j] = 1u8,
                _ => out[i][j] = 0u8,
            }
        }
    }
    out
}

#[entry]
fn main() -> ! {
    rtt_init_print!();
    let board = Board::take().unwrap();
    let mut timer = Timer::new(board.TIMER0);
    let mut display = Display::new(board.display_pins);
    let mut rng = HalRng::new(board.RNG);

    let gpiote = Gpiote::new(board.GPIOTE);
    let mut button_a = board.buttons.button_a.degrade();
    let mut button_b = board.buttons.button_b.degrade();

    unsafe { pac::NVIC::unmask(pac::Interrupt::GPIOTE) };
    pac::NVIC::unpend(pac::Interrupt::GPIOTE);

    let channel0 = gpiote.channel0();
    channel0.input_pin(&button_a).toggle().enable_interrupt();
    channel0.reset_events();

    let channel1 = gpiote.channel1();
    channel1.input_pin(&button_b).toggle().enable_interrupt();
    channel1.reset_events();

    let button_a_state = button_a.is_low().unwrap();
    BUTTON_A_STATE.store(button_a_state, Release);

    let button_b_state = button_b.is_low().unwrap();
    BUTTON_B_STATE.store(button_b_state, Release);

    let app_state = AppState {
        gpiote,
        button_a,
        button_b,
    };
    APP_STATE.init(app_state);

    let mut leds = [[0u8; 5]; 5];
    randomize(&mut leds, &mut rng);

    // NOTE: This could overflow but probably won't
    let mut frames = 5u64;

    loop {
        let pressed_a = BUTTON_A_STATE.load(Acquire);
        let pressed_b = BUTTON_B_STATE.load(Acquire);
        if frames >= 5u64 && pressed_b {
            leds = complement(&leds);
            frames = 0u64;
        } else if pressed_a {
            randomize(&mut leds, &mut rng);
        } else {
            life(&mut leds);
        }

        if done(&leds) {
            display.show(&mut timer, leds, 500);
            frames += 5u64;
        } else {
            display.show(&mut timer, leds, 100);
            frames += 1u64;
        }
    }
}
