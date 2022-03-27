use core::fmt::{Arguments, Write};

#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => ($crate::io::print(format_args!($($arg)*)));
}

#[macro_export]
macro_rules! println {
	($($arg:tt)*) => ($crate::io::print(format_args_nl!($($arg)*)));
	() => ($crate::print!("\n"));
}

pub fn print(args: Arguments) {
    let mut system_table = uefi_services::system_table();
    let stdout = unsafe { system_table.as_mut() }.stdout();
    stdout.write_fmt(args).ok();
}

#[test_case]
fn print() {
    print!("This is a message without newlines.");
}

#[test_case]
fn println() {
    println!("This is a message with a newline.");
}
