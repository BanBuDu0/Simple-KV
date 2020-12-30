
fn main() {
    let s1 = String::from("hello");
    let mut s2 = s1.clone();
    s2.insert_str(1, "aa");
    println!("'{}' ", s1);
}
