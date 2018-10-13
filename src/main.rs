extern crate serde_json;
extern crate yas_to_vscode;
use std::fs;
use yas_to_vscode::*;

fn main() {
    let src = fs::read_to_string("data/type-role").unwrap();
    println!(
        "{}",
        serde_json::to_string(&Snippet::parse(&src).unwrap().result).unwrap()
    )
}
