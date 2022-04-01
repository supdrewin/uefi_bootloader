use super::{println, str::ToCString16};
use alloc::vec::Vec;
use uefi::{
    prelude::*,
    proto::{
        device_path::DevicePath,
        loaded_image::LoadedImage,
        media::{
            file::{File, FileAttribute, FileInfo, FileMode, FileType, RegularFile},
            fs::SimpleFileSystem,
        },
    },
    table::boot::{OpenProtocolAttributes, OpenProtocolParams},
    CStr16, Char16, Error,
};

pub fn get<'a>(image_handle: Handle) -> &'a mut SimpleFileSystem {
    let system_table = uefi_services::system_table();
    let system_table = unsafe { system_table.as_ref() };
    let file_system = system_table
        .boot_services()
        .get_image_file_system(image_handle)
        .expect("BootServices::get_image_file_system failed");
    unsafe { &mut *file_system.interface.get() }
}

pub trait BootServicesExt {
    fn get_image_file_path(&self, image_handle: Handle) -> Result<&CStr16, Error>;
}

impl BootServicesExt for BootServices {
    fn get_image_file_path(&self, image_handle: Handle) -> Result<&CStr16, Error> {
        #[repr(C)]
        struct LoadedImagePart {
            revision: u32,
            parent_handle: Handle,
            system_table: *const SystemTable<Boot>,
            device_handle: Handle,
            file_path: *const DevicePath,
        }
        Ok(unsafe {
            CStr16::from_ptr(
                self.open_protocol::<LoadedImage>(
                    OpenProtocolParams {
                        handle: image_handle,
                        agent: image_handle,
                        controller: None,
                    },
                    OpenProtocolAttributes::Exclusive,
                )?
                .interface
                .get()
                .cast::<LoadedImagePart>()
                .as_ref()
                .ok_or(Status::UNSUPPORTED)?
                .file_path
                .offset(1)
                .cast::<Char16>(),
            )
        })
    }
}

pub trait FileSystem {
    fn open(&mut self, path: &str, mode: FileMode) -> Result<RegularFile, Error>;
}

impl FileSystem for SimpleFileSystem {
    fn open(&mut self, path: &str, mode: FileMode) -> Result<RegularFile, Error> {
        let path = path.to_cstring16();
        match self
            .open_volume()?
            .open(&path, mode, FileAttribute::empty())?
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
