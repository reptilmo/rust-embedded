#![no_main]
#![no_std]

use cortex_m_rt::entry;
use embedded_hal::delay::DelayNs;
use embedded_hal::digital::OutputPin;
use microbit::{
    board::Board,
    display::nonblocking::{Display, GreyscaleImage},
    hal::gpio::Level,
    hal::pac::{self, TIMER1, interrupt},
    hal::{Timer, twim},
    pac::twim0::frequency::FREQUENCY_A,
};
use panic_rtt_target as _;
use rtt_target::rtt_init_print;

use critical_section_lock_mut::LockMut;
use lsm303agr::{AccelMode, AccelOutputDataRate, Lsm303agr};

static DISPLAY: LockMut<Display<TIMER1>> = LockMut::new();
const BANG: GreyscaleImage = GreyscaleImage::new(&[
    [0, 0, 9, 0, 0],
    [0, 0, 9, 0, 0],
    [0, 0, 9, 0, 0],
    [0, 0, 0, 0, 0],
    [0, 0, 9, 0, 0],
]);

const DOT: GreyscaleImage = GreyscaleImage::new(&[
    [0, 0, 0, 0, 0],
    [0, 0, 0, 0, 0],
    [0, 0, 9, 0, 0],
    [0, 0, 0, 0, 0],
    [0, 0, 0, 0, 0],
]);

struct Filter {
    prev_x: f32,
    prev_y: f32,
    prev_z: f32,
    weight: f32,
}

impl Filter {
    fn new(weight: f32) -> Self {
        Self {
            prev_x: 0.0,
            prev_y: 0.0,
            prev_z: 0.0,
            weight,
        }
    }

    fn sample(&mut self, x: f32, y: f32, z: f32) -> (f32, f32, f32) {
        self.prev_x = self.weight * x + (1.0 - self.weight) * self.prev_x;
        self.prev_y = self.weight * y + (1.0 - self.weight) * self.prev_y;
        self.prev_z = self.weight * z + (1.0 - self.weight) * self.prev_z;

        (self.prev_x, self.prev_y, self.prev_z)
    }
}

#[entry]
fn main() -> ! {
    rtt_init_print!();
    let mut board = Board::take().unwrap();

    let display = Display::new(board.TIMER1, board.display_pins);
    DISPLAY.init(display);

    let mut speaker = board.speaker_pin.into_push_pull_output(Level::Low);
    let i2c = { twim::Twim::new(board.TWIM0, board.i2c_internal.into(), FREQUENCY_A::K100) };
    let mut timer0 = Timer::new(board.TIMER0);

    let mut sensor = Lsm303agr::new_with_i2c(i2c);
    sensor.init().unwrap();
    sensor
        .set_accel_mode_and_odr(
            &mut timer0,
            AccelMode::HighResolution,
            AccelOutputDataRate::Hz50,
        )
        .unwrap();

    let mut filter = Filter::new(0.8);

    unsafe {
        board.NVIC.set_priority(pac::Interrupt::TIMER1, 128);
        pac::NVIC::unmask(pac::Interrupt::TIMER1);
    }

    loop {
        timer0.delay_ms(1);
        if sensor.accel_status().unwrap().xyz_new_data() {
            let data = sensor.acceleration().unwrap();
            let (x, y, z) = filter.sample(
                data.x_mg() as f32 * 0.001,
                data.y_mg() as f32 * 0.001,
                data.z_mg() as f32 * 0.001,
            );
            let accel = x * x + y * y + z * z;
            if accel > 2.5 {
                DISPLAY.with_lock(|display| display.show(&BANG));
                for _ in 0u16..10 {
                    speaker.set_high().unwrap();
                    timer0.delay_us(500);
                    speaker.set_low().unwrap();
                    timer0.delay_us(500);
                }
            } else {
                DISPLAY.with_lock(|display| display.show(&DOT));
            }
        }
    }
}

#[interrupt]
fn TIMER1() {
    DISPLAY.with_lock(|display| display.handle_display_event());
}
