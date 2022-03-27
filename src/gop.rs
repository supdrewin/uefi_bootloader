use super::println;
use alloc::vec::Vec;
use core::{
    convert::Infallible,
    ops::{Deref, DerefMut},
    ptr::{slice_from_raw_parts, slice_from_raw_parts_mut},
};
use embedded_graphics::{pixelcolor::Rgb888, prelude::*};
use tinybmp::RawBmp;
use uefi::proto::console::{gop::GraphicsOutput, text::Key};

pub fn get<'a>() -> &'a mut GraphicsOutput<'a> {
    let system_table = uefi_services::system_table();
    let system_table = unsafe { system_table.as_ref() };
    let graphics_output = system_table
        .boot_services()
        .locate_protocol::<GraphicsOutput>()
        .expect("BootServices::locate_protocol failed");
    unsafe { &mut *graphics_output.get() }
}

pub trait Interaction {
    fn ask_for_a_mode(&mut self);
}

impl Interaction for GraphicsOutput<'_> {
    fn ask_for_a_mode(&mut self) {
        let mut system_table = uefi_services::system_table();
        let system_table = unsafe { system_table.as_mut() };
        let modes = self.modes().collect::<Vec<_>>();
        let (column, row) = system_table.stdout().cursor_position();
        'f: for mode in modes.iter().cycle() {
            self.set_mode(&mode)
                .expect("GraphicsOutput::set_mode failed");
            system_table
                .stdout()
                .set_cursor_position(column, row)
                .expect("Output::set_cursor_position failed");
            let (hor_res, ver_res) = mode.info().resolution();
            println!("{hor_res}x{ver_res}: Is this OK? (y)es/(n)o");
            loop {
                if let Some(Key::Printable(c)) = system_table
                    .stdin()
                    .read_key()
                    .expect("Input::read_key failed")
                {
                    match char::from(c) {
                        'y' => break 'f,
                        'n' => break,
                        _ => (),
                    }
                }
            }
        }
    }
}

pub struct FrameBuffer {
    ptr: *mut u32,
    len: usize,
    stride: u32,
    size: Size,
}

impl FrameBuffer {
    pub fn from(graphics_output: &mut GraphicsOutput) -> Self {
        let mode_info = graphics_output.current_mode_info();
        let (width, height) = mode_info.resolution();
        let mut frame_buffer = graphics_output.frame_buffer();
        Self {
            ptr: frame_buffer.as_mut_ptr().cast::<u32>(),
            len: frame_buffer.size() >> 2,
            stride: mode_info.stride() as u32,
            size: Size {
                width: width as u32,
                height: height as u32,
            },
        }
    }
}

impl Deref for FrameBuffer {
    type Target = [u32];

    fn deref(&self) -> &Self::Target {
        unsafe { &*slice_from_raw_parts(self.ptr, self.len) }
    }
}

impl DerefMut for FrameBuffer {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { &mut *slice_from_raw_parts_mut(self.ptr, self.len) }
    }
}

impl DrawTarget for FrameBuffer {
    type Color = Rgb888;

    type Error = Infallible;

    fn draw_iter<I>(&mut self, pixels: I) -> Result<(), Self::Error>
    where
        I: IntoIterator<Item = Pixel<Self::Color>>,
    {
        Ok(pixels.into_iter().for_each(|pixel| unsafe {
            if let (x @ 0.., y @ 0..) = pixel.0.into() {
                let (x, y) = (x as u32, y as u32);
                if x < self.size.width && y < self.size.height {
                    self.ptr
                        .offset((x + y * self.stride) as isize)
                        .write(pixel.1.into_storage());
                }
            }
        }))
    }
}

impl OriginDimensions for FrameBuffer {
    fn size(&self) -> Size {
        self.size
    }
}

pub trait Logo<Format, Mark> {
    fn draw(&mut self, logo: Format, mark: Mark);
}

impl Logo<RawBmp<'_>, u32> for FrameBuffer {
    fn draw(&mut self, logo: RawBmp, mark: u32) {
        let pos = Point::new(
            self.size.width as i32 - logo.header().image_size.width as i32 >> 1,
            self.size.height as i32 - logo.header().image_size.height as i32 >> 1,
        );
        logo.pixels().for_each(|pixel| unsafe {
            if let (x @ 0.., y @ 0..) = (pixel.position + pos).into() {
                let (x, y) = (x as u32, y as u32);
                if x < self.size.width && y < self.size.height && pixel.color != mark {
                    self.ptr
                        .offset((x + y * self.stride) as isize)
                        .write(pixel.color);
                }
            }
        });
    }
}
