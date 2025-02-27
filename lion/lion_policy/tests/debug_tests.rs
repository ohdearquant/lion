// These modules don't exist in the public API, so we need to use them directly
mod debug_mapper_test;
mod debug_resolver_test;

pub fn main() {
    println!("Running debug_mapper_test...");
    debug_mapper_test::main();

    println!("\nRunning debug_resolver_test...");
    debug_resolver_test::main();
}
