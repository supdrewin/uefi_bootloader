use super::println;
use core::any::type_name;

pub trait Testable {
    fn run(&self);
}

impl<T: Fn()> Testable for T {
    fn run(&self) {
        self();
        let test = type_name::<T>();
        println!("{test} ... ok");
    }
}

#[cfg(test)]
pub fn test_runner(tests: &[&dyn Testable]) {
    let system_table = uefi_services::system_table();
    let system_table = unsafe { system_table.as_ref() };
    let clock = || {
        let time = system_table
            .runtime_services()
            .get_time()
            .expect("RuntimeServices::get_time failed");
        time.day() as f64 * 60.0 * 60.0 * 24.0
            + time.hour() as f64 * 60.0 * 60.0
            + time.minute() as f64 * 60.0
            + time.second() as f64
            + time.nanosecond() as f64 / 1e9
    };
    println!("running {} tests", tests.len());
    let begin = clock();
    tests.iter().for_each(|test| {
        test.run();
    });
    let end = clock();
    let elapsed = end - begin;
    println!("finished in {elapsed:.2}s");
}
