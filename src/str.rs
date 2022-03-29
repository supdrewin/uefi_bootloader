use uefi::CString16;

pub trait ToCString16 {
    fn to_cstring16(&self) -> CString16;
}

impl ToCString16 for str {
    fn to_cstring16(&self) -> CString16 {
        CString16::try_from(self).expect("CString16::try_from failed")
    }
}
