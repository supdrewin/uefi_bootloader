#![feature(abi_efiapi, custom_test_frameworks, format_args_nl)]
#![no_main]
#![no_std]
#![reexport_test_harness_main = "test_main"]
#![test_runner(test::test_runner)]

mod cfg;
mod fs;
mod gop;
mod io;
mod map;
mod str;
mod test;

#[macro_use]
extern crate alloc;

use alloc::string::ToString;
use cfg::{Config, ConfigData, DEFAULT_LOGO};
use embedded_graphics::{
    mono_font::{ascii::FONT_10X20, MonoTextStyle},
    pixelcolor::Rgb888,
    prelude::*,
    primitives::{PrimitiveStyle, PrimitiveStyleBuilder, Rectangle},
    text::{Alignment, Text},
};
use fs::{BootServicesExt, FileExt, FileSystem};
use gop::{DrawMasked, FrameBuffer, Interaction, BACKGROUND_COLOR, STROKE_COLOR};
use tinybmp::Bmp;
use uefi::{
    prelude::*,
    proto::{
        console::{
            gop::GraphicsOutput,
            text::{Key, ScanCode},
        },
        media::file::FileMode,
    },
    table::runtime::ResetType,
    Result,
};

#[entry]
fn main(image_handle: Handle, mut system_table: SystemTable<Boot>) -> Status {
    uefi_services::init(&mut system_table)?;
    let graphics_output = gop::get();
    let file_system = fs::get(image_handle);
    let config_path = system_table
        .boot_services()
        .get_image_file_path(image_handle)
        .expect("Failed to get image file path")
        .to_string();
    let config_path = config_path
        .rsplit_once('.')
        .expect("String::rsplit_once failed");
    let config_path = config_path.0.to_string() + ".json";
    let config_file = file_system.open(&config_path, FileMode::CreateReadWrite)?;
    let mut config_data = ConfigData::default();
    if let Ok(mut config) = Config::new(config_file) {
        let resolution: (usize, usize) = config.resolution.into();
        let result = graphics_output
            .modes()
            .find(|mode| resolution == mode.info().resolution());
        if let Some(mode) = result {
            graphics_output.set_mode(&mode)?;
        } else if let Ok(_) = graphics_output.set_resolution() {
            let info = graphics_output.current_mode_info();
            config.resolution = info.resolution().into();
        }
        config_data = config.clone();
    }
    let mut frame_buffer = FrameBuffer::from(&mut *graphics_output);
    let mut draw_logo = || {
        frame_buffer.clear(config_data.background.into())?;
        let mut draw_logo = |bytes: &[u8]| {
            let logo = Bmp::<Rgb888>::from_slice(&bytes).or(Err(Status::UNSUPPORTED))?;
            let offset = Point::new(
                frame_buffer.size().width as i32 - logo.size().width as i32 >> 1,
                frame_buffer.size().height as i32 - logo.size().height as i32 >> 1,
            );
            frame_buffer.draw_masked(logo.pixels(), Rgb888::BLACK, offset)
        };
        match file_system.open(&config_data.logo_path, FileMode::Read) {
            Ok(mut bitmap) => draw_logo(&bitmap.load()?),
            Err(_) => draw_logo(DEFAULT_LOGO),
        }
    };
    draw_logo()?;
    #[cfg(test)]
    test_main();
    let key_event = system_table.stdin().wait_for_key_event();
    let key_event = unsafe { key_event.unsafe_clone() };
    let mut events = [key_event];
    while let Ok(_) = system_table.boot_services().wait_for_event(&mut events) {
        if let Some(Key::Special(ScanCode::ESCAPE)) = system_table.stdin().read_key()? {
            power_options(graphics_output)?;
            draw_logo()?;
        }
    }
    Status::ABORTED
}

fn power_options(graphics_output: &mut GraphicsOutput) -> Result {
    let mut frame_buffer = FrameBuffer::from(&mut *graphics_output);
    let (x, y) = graphics_output.current_mode_info().resolution();
    let center = Point::new(x as i32 >> 1, y as i32 >> 1);
    Rectangle::new(
        Point::new(center.x - 100, center.y - 150),
        Size::new(200, 300),
    )
    .into_styled(PrimitiveStyle::with_fill(BACKGROUND_COLOR))
    .draw(&mut frame_buffer)?;
    Rectangle::new(
        Point::new(center.x - 90, center.y - 140),
        Size::new(180, 280),
    )
    .into_styled(
        PrimitiveStyleBuilder::new()
            .stroke_color(STROKE_COLOR)
            .stroke_width(1)
            .build(),
    )
    .draw(&mut frame_buffer)?;
    let mut character_style = MonoTextStyle::new(&FONT_10X20, Rgb888::RED);
    character_style.background_color = Some(BACKGROUND_COLOR);
    Text::with_alignment(
        "Options",
        Point::new(center.x, center.y - 135),
        character_style,
        Alignment::Center,
    )
    .draw(&mut frame_buffer)?;
    character_style.text_color = Some(Rgb888::BLUE);
    Text::with_alignment(
        "<Enter>",
        Point::new(center.x, center.y + 120),
        character_style,
        Alignment::Center,
    )
    .draw(&mut frame_buffer)?;
    let dialog_box = Rectangle::new(
        Point::new(center.x - 80, center.y - 100),
        Size::new(160, 200),
    )
    .into_styled(PrimitiveStyle::with_fill(BACKGROUND_COLOR));
    let mut system_table = uefi_services::system_table();
    let system_table = unsafe { system_table.as_mut() };
    let key_event = system_table.stdin().wait_for_key_event();
    let key_event = unsafe { key_event.unsafe_clone() };
    let mut events = [key_event];
    let position = Point::new(center.x, center.y - 35);
    let texts = ["Continue", "Reboot", "Shutdown"];
    let mut index = 0;
    let index = 'outer: loop {
        dialog_box.draw(&mut frame_buffer)?;
        let mut position = position;
        texts.iter().enumerate().try_for_each(|(i, text)| {
            character_style.text_color = Some(match i == index % texts.len() {
                false => Rgb888::BLACK,
                true => Rgb888::BLUE,
            });
            Text::with_alignment(&text, position, character_style, Alignment::Center)
                .draw(&mut frame_buffer)
                .and_then(|_| Ok(position.y += 30))
        })?;
        while let Ok(_) = system_table.boot_services().wait_for_event(&mut events) {
            if let Some(key) = system_table.stdin().read_key()? {
                match key {
                    Key::Printable(c) if '\r' == c.into() => {
                        let index = index % texts.len();
                        break 'outer index;
                    }
                    Key::Special(c) => match c {
                        ScanCode::ESCAPE => break 'outer 0,
                        ScanCode::UP => {
                            index += texts.len() - 1;
                            index %= texts.len();
                            break;
                        }
                        ScanCode::DOWN => {
                            index += 1;
                            break;
                        }
                        _ => (),
                    },
                    _ => (),
                }
            }
        }
    };
    let runtime_services = system_table.runtime_services();
    let reset = |rt: ResetType| {
        runtime_services.reset(rt, Status::SUCCESS, None);
    };
    match index {
        1 => reset(ResetType::Cold),
        2 => reset(ResetType::Shutdown),
        _ => Ok(()),
    }
}
