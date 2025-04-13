#![feature(unsafe_cell_access)]

pub mod vm;
pub mod value;
pub mod building;
pub mod variable;
pub mod instruction;
pub mod interface;

pub fn add(left: u64, right: u64) -> u64 {
    left + right
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let result = add(2, 2);
        assert_eq!(result, 4);
    }
}
