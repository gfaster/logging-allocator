use logging_allocator::{run_guarded, LoggingAllocator};

#[global_allocator]
static ALLOC: LoggingAllocator = LoggingAllocator::new(true);

fn main() {
    simple_logger::init().unwrap();

    let mut vec = vec![0; 4];
    run_guarded(|| eprintln!("Inserting some numbers"));
    vec.extend(&[1, 2, 3, 4, 5]);
    run_guarded(|| eprintln!("Cloning the vector"));
    let _clone = vec.clone();
    run_guarded(|| eprintln!("Dropping the original vector"));
    drop(vec);
    run_guarded(|| eprintln!("Finished!"));
}
