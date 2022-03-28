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

use alloc::{string::ToString, vec::Vec};
use cfg::Config;
use core::{mem, str};
use embedded_graphics::{pixelcolor::Rgb888, prelude::*};
use fs::FileSystem;
use gop::{DrawMarked, FrameBuffer, Interaction, Resolution};
use tinybmp::Bmp;
use uefi::{
    prelude::*,
    proto::{
        console::text::{Key, ScanCode},
        media::file::FileMode,
    },
    table::runtime::ResetType,
    CString16,
};

#[entry]
fn main(_image: Handle, mut system_table: SystemTable<Boot>) -> Status {
    uefi_services::init(&mut system_table)?;
    let mut config = Config::new(r"\efi\boot\boot.cfg");
    let graphics_output = gop::get();
    match config.get("resolution").and_then(|(resolution, _)| {
        resolution.split_once('x').and_then(|(hor_res, ver_res)| {
            let resolution = (
                hor_res.parse::<usize>().expect("str::parse failed"),
                ver_res.parse::<usize>().expect("str::parse failed"),
            );
            graphics_output
                .modes()
                .find(|mode| resolution == mode.info().resolution())
        })
    }) {
        Some(mode) => graphics_output.set_mode(&mode)?,
        None => {
            graphics_output.ask_for_a_mode();
            let resolution = graphics_output.current_mode_info().resolution();
            let resolution = Resolution::from(resolution).to_string();
            config.insert("resolution".to_string(), (resolution, true));
        }
    }
    let mut frame_buffer = FrameBuffer::from(graphics_output);
    if let Some((background, _)) = config.get("background") {
        let color = background
            .split(',')
            .map(|num| num.trim().parse().expect("str::parse failed"))
            .collect::<Vec<u8>>();
        assert_eq!(color.len(), 3);
        frame_buffer.clear(Rgb888::new(color[0], color[1], color[2]))?;
    }
    if let Some((path, _)) = config.get("logo_path") {
        let bytes = fs::get().open(path, FileMode::Read).load();
        let logo = Bmp::<Rgb888>::from_slice(&bytes).expect("Bmp::from_slice failed");
        let offset = Point::new(
            frame_buffer.size().width as i32 - logo.size().width as i32 >> 1,
            frame_buffer.size().height as i32 - logo.size().height as i32 >> 1,
        );
        frame_buffer.draw_marked(logo.pixels(), Rgb888::BLACK, offset)?;
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
        if let Some(Key::Special(ScanCode::ESCAPE)) = system_table
            .stdin()
            .read_key()
            .expect("Input::read_key failed")
        {
            mem::drop(config);
            system_table
                .runtime_services()
                .reset(ResetType::Shutdown, Status::SUCCESS, None);
        }
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
