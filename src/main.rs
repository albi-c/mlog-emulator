use std::io::{stdin, stdout};
use emulator::interface::run_from_json;

fn main() {
    run_from_json(stdin(), stdout());
}
