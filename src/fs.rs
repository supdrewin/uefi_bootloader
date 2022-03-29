use super::{println, str::ToCString16};
use alloc::vec::Vec;
use uefi::{
    proto::media::{
        file::{File, FileAttribute as FileAttr, FileInfo, FileMode, FileType, RegularFile},
        fs::SimpleFileSystem,
    },
    Error, Status,
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
    fn open(&mut self, path: &str, mode: FileMode) -> Result<RegularFile, Error>;
}

impl FileSystem for SimpleFileSystem {
    fn open(&mut self, path: &str, mode: FileMode) -> Result<RegularFile, Error> {
        let path = path.to_cstring16();
        match self
            .open_volume()?
            .open(&path, mode, FileAttr::empty())?
            .into_type()?
        {
            FileType::Regular(handle) => Ok(handle),
            FileType::Dir(_) => {
                println!("{path} is a directory");
                Err(Error::from(Status::UNSUPPORTED))
            }
        }
    }
}

pub trait FileExt {
    fn load(&mut self) -> Result<Vec<u8>, Error>;

    fn replace(&mut self, buffer: &[u8]) -> Result<(), Error<usize>>;
}

impl FileExt for RegularFile {
    fn load(&mut self) -> Result<Vec<u8>, Error> {
        let info = self.get_boxed_info::<FileInfo>()?;
        let mut buffer = vec![Default::default(); info.file_size() as usize];
        self.read(&mut buffer).expect("RegularFile::read failed");
        Ok(buffer)
    }

    fn replace(&mut self, buffer: &[u8]) -> Result<(), Error<usize>> {
        let len = self
            .get_boxed_info::<FileInfo>()
            .expect("File::get_boxed_info failed")
            .file_size() as usize;
        self.set_position(Default::default())
            .expect("RegularFile::set_position failed");
        self.write(buffer)?;
        if len > buffer.len() {
            let buffer = vec![b' '; len - buffer.len()];
            self.write(&buffer)?;
        }
        Ok(())
    }
}
