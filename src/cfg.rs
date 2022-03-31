use super::{
    fs::FileExt,
    gop::{Color, Resolution},
};
use alloc::{str, string::String};
use core::ops::{Deref, DerefMut};
use serde::{Deserialize, Serialize};
use uefi::{proto::media::file::RegularFile, Error};

pub const CONFIG_PATH: &str = r"\efi\boot\boot.json";
pub const DEFAULT_LOGO: &[u8] = include_bytes!("boot.bmp");

#[derive(Debug, Default, PartialEq, Eq, Deserialize, Serialize)]
pub struct ConfigData {
    pub background: Color,
    pub logo_path: String,
    pub resolution: Resolution,
}

pub struct Config {
    config_data: ConfigData,
    config_file: RegularFile,
}

impl Config {
    pub fn new(mut config_file: RegularFile) -> Result<Self, Error> {
        Ok(Self {
            config_data: serde_json::from_slice::<ConfigData>(&config_file.load()?)
                .unwrap_or_default(),
            config_file,
        })
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
        self.config_file
            .replace(
                serde_json::to_string_pretty(&self.config_data)
                    .expect("serde_json::to_string failed")
                    .as_bytes(),
            )
            .expect("File::replace failed");
    }
}
