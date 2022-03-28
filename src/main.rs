#![feature(abi_efiapi, custom_test_frameworks, format_args_nl)]
#![no_main]
#![no_std]
#![reexport_test_harness_main = "test_main"]
#![test_runner(test::test_runner)]

mod cfg;
mod fs;
mod gop;
mod io;
mod test;

#[macro_use]
extern crate alloc;

use embedded_graphics::{
    pixelcolor::Rgb888,
    prelude::*,
    primitives::{PrimitiveStyle, Rectangle},
};
use fs::FileSystem;
use gop::{DrawMarked, FrameBuffer, Interaction};
use tinybmp::Bmp;
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
    let logo = Bmp::<Rgb888>::from_slice(&bytes).expect("Bmp::from_slice failed");
    let offset = Point::new(
        frame_buffer.size().width as i32 - logo.size().width as i32 >> 1,
        frame_buffer.size().height as i32 - logo.size().height as i32 >> 1,
    );
    frame_buffer.draw_marked(logo.pixels(), Rgb888::BLACK, offset)?;
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
