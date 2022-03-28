use super::fs::{self, File, FileSystem};
use alloc::{
    str,
    string::{String, ToString},
    vec::Vec,
};
use core::ops::{Deref, DerefMut};
use hashbrown::HashMap;
use uefi::proto::media::file::FileMode;

pub struct Config {
    config: HashMap<String, (String, bool)>,
    file: File,
}

impl Config {
    pub fn new(path: &str) -> Self {
        let mut file = fs::get().open(path, FileMode::CreateReadWrite);
        let content = file.load();
        let content = str::from_utf8(&content).expect("str::from_utf8 failed");
        let iter = content
            .lines()
            .filter_map(|line| line.trim().split('#').next())
            .filter(|line| !line.is_empty())
            .map(|line| match line.split_once('=') {
                Some((k, v)) => (k.trim().to_string(), (v.trim().to_string(), false)),
                None => panic!("invalid configuration content"),
            });
        Self {
            config: HashMap::from_iter(iter),
            file,
        }
    }
}

impl Deref for Config {
    type Target = HashMap<String, (String, bool)>;

    fn deref(&self) -> &<Self as Deref>::Target {
        &self.config
    }
}

impl DerefMut for Config {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.config
    }
}

impl Drop for Config {
    fn drop(&mut self) {
        self.iter()
            .filter(|(_, (_, is_changed))| *is_changed)
            .map(|(k, (v, _))| format!("{k} = {v}\n"))
            .collect::<Vec<_>>()
            .into_iter()
            .for_each(|buffer| {
                let buffer = buffer.as_bytes();
                self.file.write(buffer).ok();
            });
    }
}
