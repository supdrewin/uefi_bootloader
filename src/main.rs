#![feature(abi_efiapi, custom_test_frameworks, format_args_nl)]
#![no_main]
#![no_std]
#![reexport_test_harness_main = "test_main"]
#![test_runner(test::test_runner)]

mod cfg;
mod fs;
mod gop;
mod io;
mod str;
mod test;

#[macro_use]
extern crate alloc;

use cfg::Config;
use embedded_graphics::{pixelcolor::Rgb888, prelude::*};
use fs::{FileExt, FileSystem};
use gop::{DrawMasked, FrameBuffer, Interaction};
use tinybmp::Bmp;
use uefi::{
    prelude::*,
    proto::{
        console::text::{Key, ScanCode},
        media::file::FileMode,
    },
    table::runtime::ResetType,
};

#[entry]
fn main(_image: Handle, mut system_table: SystemTable<Boot>) -> Status {
    uefi_services::init(&mut system_table)?;
    if let Ok(mut config) = Config::new(r"\efi\boot\boot.json") {
        let graphics_output = gop::get();
        let resolution = <(usize, usize)>::from(config.resolution);
        let result = graphics_output
            .modes()
            .find(|mode| resolution == mode.info().resolution());
        if let Some(mode) = result {
            graphics_output.set_mode(&mode)?
        } else {
            graphics_output.ask_for_a_mode();
            let info = graphics_output.current_mode_info();
            config.resolution = info.resolution().into();
        }
        let mut frame_buffer = FrameBuffer::from(graphics_output);
        frame_buffer.clear(config.background.into())?;
        if let Ok(mut bitmap) = fs::get().open(&config.logo_path, FileMode::Read) {
            let bytes = bitmap.load()?;
            let logo = Bmp::<Rgb888>::from_slice(&bytes).expect("Bmp::from_slice failed");
            let offset = Point::new(
                frame_buffer.size().width as i32 - logo.size().width as i32 >> 1,
                frame_buffer.size().height as i32 - logo.size().height as i32 >> 1,
            );
            frame_buffer.draw_masked(logo.pixels(), Rgb888::BLACK, offset)?;
        }
    }
    #[cfg(test)]
    test_main();
    let key_event = system_table.stdin().wait_for_key_event();
    let key_event = unsafe { key_event.unsafe_clone() };
    let mut events = [key_event];
    loop {
        system_table
            .boot_services()
            .wait_for_event(&mut events)
            .expect("BootServices::wait_for_event failed");
        match system_table.stdin().read_key()? {
            Some(Key::Special(ScanCode::ESCAPE)) => {
                let runtime_services = system_table.runtime_services();
                runtime_services.reset(ResetType::Shutdown, Status::SUCCESS, None);
            }
            Some(Key::Special(c)) => print!("{c:?}"),
            Some(Key::Printable(c)) => print!("{c}"),
            None => (),
        }
    }
}
