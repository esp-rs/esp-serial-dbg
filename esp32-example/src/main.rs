#![no_std]
#![no_main]
#![feature(asm_experimental_arch)]

use esp32_hal::{
    clock::ClockControl, peripherals::Peripherals, prelude::*, timer::TimerGroup, Delay, Rtc, Uart,
};
use esp_backtrace as _;
use esp_println::println;

#[entry]
fn main() -> ! {
    let peripherals = Peripherals::take();
    let system = peripherals.DPORT.split();
    let clocks = ClockControl::boot_defaults(system.clock_control).freeze();

    // Disable the RTC and TIMG watchdog timers
    let mut rtc = Rtc::new(peripherals.RTC_CNTL);
    let timer_group0 = TimerGroup::new(peripherals.TIMG0, &clocks);
    let mut wdt0 = timer_group0.wdt;
    let timer_group1 = TimerGroup::new(peripherals.TIMG1, &clocks);
    let mut wdt1 = timer_group1.wdt;

    rtc.rwdt.disable();
    wdt0.disable();
    wdt1.disable();

    esp_serial_dbg::init(Uart::new(peripherals.UART0));

    let mut delay = Delay::new(&clocks);

    let mut i = 0;
    println!("ok");

    loop {
        some_ram_function(i);
        i = i.wrapping_add(1);
        delay.delay_ms(1500u32);
        some_function(i);
        i = i.wrapping_add(1);
        delay.delay_ms(1500u32);
    }
}

fn some_function(param: u32) {
    println!("hello {}", param);
}

#[ram]
fn some_ram_function(param: u32) {
    // also SW breakpoints work
    println!("hello from ram function! {}", param);
}
