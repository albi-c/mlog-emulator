#![feature(unsafe_cell_access)]

use std::io::{stdin, stdout};
use crate::interface::run_from_json;

mod vm;
mod value;
mod building;
mod variable;
mod instruction;
mod interface;

fn main() {
    run_from_json(stdin(), stdout());
}
