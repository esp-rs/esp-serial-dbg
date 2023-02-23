#![no_std]
#![no_main]

use core::cell::RefCell;

use critical_section::Mutex;
use esp32c2_hal::{
    clock::ClockControl,
    interrupt,
    peripherals::{Peripherals, TIMG0},
    prelude::*,
    timer::{Timer, Timer0, TimerGroup},
    Delay, Rtc, Uart,
};
use esp_backtrace as _;
use esp_println::println;

static TIMER0: Mutex<RefCell<Option<Timer<Timer0<TIMG0>>>>> = Mutex::new(RefCell::new(None));

#[entry]
fn main() -> ! {
    let peripherals = Peripherals::take();
    let system = peripherals.SYSTEM.split();
    let clocks = ClockControl::boot_defaults(system.clock_control).freeze();

    // Disable the RTC and TIMG watchdog timers
    let mut rtc = Rtc::new(peripherals.RTC_CNTL);
    let timer_group0 = TimerGroup::new(peripherals.TIMG0, &clocks);
    let mut wdt0 = timer_group0.wdt;

    rtc.swd.disable();
    rtc.rwdt.disable();
    wdt0.disable();

    esp_serial_dbg::init(Uart::new(peripherals.UART0));

    let mut delay = Delay::new(&clocks);

    let mut timer0 = timer_group0.timer0;
    interrupt::enable(
        esp32c2_hal::peripherals::Interrupt::TG0_T0_LEVEL,
        interrupt::Priority::Priority1,
    )
    .unwrap();
    timer0.start(2500u64.millis());
    timer0.listen();

    critical_section::with(|cs| {
        TIMER0.borrow_ref_mut(cs).replace(timer0);
    });

    let mut i = 0;
    println!("ok");

    // breakpoints here work fine
    loop {
        some_function(i);
        i = i.wrapping_add(1);
        delay.delay_ms(1500u32);
        some_function(i);
        i = i.wrapping_add(1);
        delay.delay_ms(1500u32);
        critical_section::with(|_| {
            // also here breakpoints work
            some_ram_function(i);
        });
    }
}

fn some_function(param: u32) {
    println!("hello {}", param);
}

#[ram]
fn some_ram_function(param: u32) {
    // also SW breakpoints work
    let mut x = 0;
    println!("hello from ram function! {}", param);
    x += 1;
    println!("hello from ram function! x={}", x);
}

fn called_by_interrupt_handler() {
    esp_println::println!("Interrupt 1, again!");
}

#[interrupt]
fn TG0_T0_LEVEL() {
    // unfortunately no breakpoints inside interrupt handlers
    esp_println::println!("Interrupt 1");
    called_by_interrupt_handler();

    critical_section::with(|cs| {
        let mut timer0 = TIMER0.borrow_ref_mut(cs);
        let timer0 = timer0.as_mut().unwrap();

        timer0.clear_interrupt();
        timer0.start(1000u64.millis());
    });
}
