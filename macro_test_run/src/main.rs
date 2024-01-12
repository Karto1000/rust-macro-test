use macro_test::from_file;
use macro_test_traits::LoadStatic;
use serde::Deserialize;

#[from_file(path = "./test.json", is_static = true)]
#[derive(Debug, Deserialize)]
struct Test {
    test: Option<i64>,
    test2: Vec<Test2>,
}

#[derive(Debug, Deserialize)]
struct Test2 {
    test2: String,
}

fn main() {
    println!("cargo:rerun-if-changed={}", "./test.json");

    let test = Test::load_static();
    println!("{:?}", test.test2);
    println!("awd {:?}", test);
}

