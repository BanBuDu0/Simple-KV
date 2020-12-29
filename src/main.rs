fn main() {
    let s1 = String::from("hello");
    let mut s2 = s1.clone();
    s2.insert_str(1, "aa");
    println!("'{}' ", s1);
}

fn calculate_length(s: &String) -> bool {
    let a: String = s.clone();
    println!("'{}' ", a);
    s.len() == 2
}