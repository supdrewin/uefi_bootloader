use alloc::{string::ToString, vec::Vec};
use core::{
    fmt::{Display, Formatter, Result as FmtResult},
    ops::{Deref, DerefMut},
    ptr::{slice_from_raw_parts, slice_from_raw_parts_mut},
};
use embedded_graphics::{
    mono_font::{ascii::FONT_10X20, MonoTextStyle},
    pixelcolor::Rgb888,
    prelude::*,
    primitives::{PrimitiveStyle, PrimitiveStyleBuilder, Rectangle},
    text::{Alignment, Text},
};
use serde::{Deserialize, Serialize};
use uefi::{
    proto::console::{
        gop::GraphicsOutput,
        text::{Key, ScanCode},
    },
    Error, Result as UefiResult, Status,
};

pub const BACKGROUND_COLOR: Rgb888 = Rgb888::new(168, 154, 132);
pub const STROKE_COLOR: Rgb888 = Rgb888::new(40, 40, 40);

pub fn get<'a>() -> &'a mut GraphicsOutput<'a> {
    let system_table = uefi_services::system_table();
    let system_table = unsafe { system_table.as_ref() };
    let graphics_output = system_table
        .boot_services()
        .locate_protocol::<GraphicsOutput>()
        .expect("BootServices::locate_protocol failed");
    unsafe { &mut *graphics_output.get() }
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl From<Rgb888> for Color {
    fn from(color: Rgb888) -> Self {
        Color {
            r: color.r(),
            g: color.g(),
            b: color.b(),
        }
    }
}

impl From<Color> for Rgb888 {
    fn from(color: Color) -> Self {
        let Color { r, g, b } = color;
        Rgb888::new(r, g, b)
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
pub struct Resolution {
    pub width: usize,
    pub height: usize,
}

impl Display for Resolution {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        let Self { width, height } = self;
        f.write_fmt(format_args!("{width}x{height}"))
    }
}

impl From<(usize, usize)> for Resolution {
    fn from(res: (usize, usize)) -> Self {
        Self {
            width: res.0,
            height: res.1,
        }
    }
}

impl From<Resolution> for (usize, usize) {
    fn from(res: Resolution) -> Self {
        (res.width, res.height)
    }
}

pub trait Interaction {
    fn set_resolution(&mut self) -> UefiResult;
}

impl Interaction for GraphicsOutput<'_> {
    fn set_resolution(&mut self) -> UefiResult {
        let mut frame_buffer = FrameBuffer::from(&mut *self);
        let (x, y) = self.current_mode_info().resolution();
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
            "Resolution",
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
        let modes = self.modes().collect::<Vec<_>>();
        let bound = modes.len().min(5);
        let position = Point::new(center.x, center.y - 35 * (bound >> 1) as i32);
        let mut index = 0;
        loop {
            dialog_box.draw(&mut frame_buffer)?;
            let mut position = position;
            (0..bound).into_iter().try_for_each(|i| {
                character_style.text_color = Some(match i == bound >> 1 {
                    false => Rgb888::BLACK,
                    true => Rgb888::BLUE,
                });
                let index = (index + i) % modes.len();
                let resolution = modes[index].info().resolution();
                let text = Resolution::from(resolution).to_string();
                Text::with_alignment(&text, position, character_style, Alignment::Center)
                    .draw(&mut frame_buffer)
                    .and_then(|_| Ok(position.y += 30))
            })?;
            while let Ok(_) = system_table.boot_services().wait_for_event(&mut events) {
                if let Some(key) = system_table.stdin().read_key()? {
                    match key {
                        Key::Printable(c) if '\r' == c.into() => {
                            let mode = &modes[((bound >> 1) + index) % modes.len()];
                            return self.set_mode(mode);
                        }
                        Key::Special(c) => match c {
                            ScanCode::ESCAPE => return Err(Error::from(Status::ABORTED)),
                            ScanCode::UP => {
                                index += modes.len() - 1;
                                index %= modes.len();
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
        }
    }
}

pub trait DrawMasked: DrawTarget + Sized {
    fn draw_masked<I>(
        &mut self,
        pixels: I,
        mask: Self::Color,
        offset: Point,
    ) -> Result<(), Self::Error>
    where
        I: IntoIterator<Item = Pixel<Self::Color>>,
    {
        pixels
            .into_iter()
            .filter(|pixel| pixel.1 != mask)
            .translated(offset)
            .draw(self)
    }
}

pub struct FrameBuffer {
    ptr: *mut u32,
    len: usize,
    stride: u32,
    size: Size,
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

impl DrawMasked for FrameBuffer {}

impl DrawTarget for FrameBuffer {
    type Color = Rgb888;

    type Error = Error;

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

impl From<&mut GraphicsOutput<'_>> for FrameBuffer {
    fn from(graphics_output: &mut GraphicsOutput) -> Self {
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

impl OriginDimensions for FrameBuffer {
    fn size(&self) -> Size {
        self.size
    }
}
