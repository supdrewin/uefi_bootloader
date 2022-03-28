use super::ToCString16;
use alloc::vec::Vec;
use core::ops::{Deref, DerefMut};
use uefi::{
    proto::media::{
        file::{File as UefiFile, FileAttribute, FileInfo, FileMode, FileType, RegularFile},
        fs::SimpleFileSystem,
    },
    CString16,
};

pub fn get<'a>() -> &'a mut SimpleFileSystem {
    let system_table = uefi_services::system_table();
    let system_table = unsafe { system_table.as_ref() };
    let file_system = system_table
        .boot_services()
        .locate_protocol::<SimpleFileSystem>()
        .expect("BootServices::locate_protocol failed");
    unsafe { &mut *file_system.get() }
}

pub trait FileSystem {
    fn open(&mut self, path: &str, mode: FileMode) -> File;
}

impl FileSystem for SimpleFileSystem {
    fn open(&mut self, path: &str, mode: FileMode) -> File {
        let path = path.to_cstring16();
        match self
            .open_volume()
            .expect("SimpleFileSystem::open_volume failed")
            .open(&path, mode, FileAttribute::empty())
            .expect("File::open failed")
            .into_type()
            .expect("FileHandle::into_type failed")
        {
            FileType::Regular(handle) => File { handle, path },
            FileType::Dir(_) => panic!("{path} is a directory"),
        }
    }
}

pub struct File {
    handle: RegularFile,
    path: CString16,
}

impl File {
    pub fn load(&mut self) -> Vec<u8> {
        let mut buffer = vec![Default::default(); 80 + self.path.num_bytes()];
        let info = self
            .get_info::<FileInfo>(&mut buffer)
            .expect("RegularFile::get_info failed");
        let mut buffer = vec![Default::default(); info.file_size() as usize];
        self.read(&mut buffer).expect("RegularFile::read failed");
        buffer
    }
}

impl Deref for File {
    type Target = RegularFile;

    fn deref(&self) -> &Self::Target {
        &self.handle
    }
}

impl DerefMut for File {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.handle
    }
}
