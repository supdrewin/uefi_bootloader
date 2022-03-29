use super::{
    fs::{self, FileExt, FileSystem},
    gop::Resolution,
    println,
};
use alloc::{
    str,
    string::{String, ToString},
};
use core::ops::{Deref, DerefMut};
use embedded_graphics::pixelcolor::{Rgb888, RgbColor};
use uefi::{
    proto::media::file::{FileMode, RegularFile},
    Error,
};

#[derive(Default)]
pub struct ConfigData {
    pub background: Rgb888,
    pub logo_path: String,
    pub resolution: Resolution,
}

pub struct Config {
    config_data: ConfigData,
    config_file: RegularFile,
}

impl Config {
    pub fn new(path: &str) -> Result<Self, Error> {
        let mut this = Self {
            config_data: Default::default(),
            config_file: fs::get().open(path, FileMode::CreateReadWrite)?,
        };
        let content = this.config_file.load()?;
        str::from_utf8(&content)
            .expect("str::from_utf8 failed")
            .lines()
            .filter_map(|line| line.trim().split('#').next())
            .filter(|line| !line.is_empty())
            .map(|line| match line.split_once('=') {
                Some((key, value)) => (key.trim(), value.trim()),
                None => panic!("Invalid config content: {line}"),
            })
            .for_each(|(key, value)| match key {
                "background" => {
                    let mut color = value.split(',').map(|num| num.trim().parse());
                    if let (Some(Ok(r)), Some(Ok(g)), Some(Ok(b)), None) =
                        (color.next(), color.next(), color.next(), color.next())
                    {
                        this.config_data.background = Rgb888::new(r, g, b);
                    }
                }
                "logo_path" => this.config_data.logo_path = value.to_string(),
                "resolution" => {
                    if let Some((hor_res, ver_res)) = value.split_once('x') {
                        if let (Ok(hor_res), Ok(ver_res)) = (hor_res.parse(), ver_res.parse()) {
                            this.config_data.resolution = (hor_res, ver_res).into();
                        }
                    }
                }
                key => println!("Invalid config item: {key}"),
            });
        Ok(this)
    }
}

impl Deref for Config {
    type Target = ConfigData;

    fn deref(&self) -> &Self::Target {
        &self.config_data
    }
}

impl DerefMut for Config {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.config_data
    }
}

impl Drop for Config {
    fn drop(&mut self) {
        let Self {
            config_data:
                ConfigData {
                    background,
                    logo_path,
                    resolution,
                },
            config_file,
        } = self;
        let buffer = format!(
            "\
background = {}, {}, {}
logo_path = {logo_path}
resolution = {resolution}
",
            background.r(),
            background.g(),
            background.b()
        );
        config_file
            .replace(buffer.as_bytes())
            .expect("File::replace failed");
    }
}
