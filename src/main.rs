#![feature(abi_efiapi, custom_test_frameworks, format_args_nl)]
#![no_main]
#![no_std]
#![reexport_test_harness_main = "test_main"]
#![test_runner(test_runner)]

mod fs;
mod gop;
mod io;

#[macro_use]
extern crate alloc;

use core::any;
use embedded_graphics::{
    prelude::*,
    primitives::{PrimitiveStyle, Rectangle},
};
use fs::FileSystem;
use gop::{FrameBuffer, Interaction, Logo};
use tinybmp::RawBmp;
use uefi::{prelude::*, CString16};
use x86_64::instructions;

#[entry]
fn main(_: Handle, mut system_table: SystemTable<Boot>) -> Status {
    uefi_services::init(&mut system_table)?;
    let graphics_output = gop::get();
    graphics_output.ask_for_a_mode();
    let mut frame_buffer = FrameBuffer::from(graphics_output);
    Rectangle::new(Default::default(), frame_buffer.size())
        .into_styled(PrimitiveStyle::with_fill(RgbColor::CYAN))
        .draw(&mut frame_buffer)
        .expect("Drawable::draw failed");
    let bytes = fs::get().open(r"\efi\boot\boot.bmp").load();
    let logo = RawBmp::from_slice(&bytes).expect("RawBmp::from_slice failed");
    frame_buffer.draw(logo, Default::default());
    #[cfg(test)]
    test_main();
    loop {
        instructions::hlt();
    }
}

trait ToCString16 {
    fn to_cstring16(&self) -> CString16;
}

impl ToCString16 for str {
    fn to_cstring16(&self) -> CString16 {
        CString16::try_from(self).expect("CString16::try_from failed")
    }
}

trait Testable {
    fn run(&self);
}

impl<T: Fn()> Testable for T {
    fn run(&self) {
        self();
        let test = any::type_name::<T>();
        println!("{test} ... ok");
    }
}

#[cfg(test)]
fn test_runner(tests: &[&dyn Testable]) {
    let system_table = uefi_services::system_table();
    let system_table = unsafe { system_table.as_ref() };
    let clock = || {
        let time = system_table
            .runtime_services()
            .get_time()
            .expect("RuntimeServices::get_time failed");
        time.day() as f64 * 60.0 * 60.0 * 24.0
            + time.hour() as f64 * 60.0 * 60.0
            + time.minute() as f64 * 60.0
            + time.second() as f64
            + time.nanosecond() as f64 / 1e9
    };
    println!("running {} tests", tests.len());
    let begin = clock();
    tests.iter().for_each(|test| {
        test.run();
    });
    let end = clock();
    let elapsed = end - begin;
    println!("finished in {elapsed:.2}s");
}
