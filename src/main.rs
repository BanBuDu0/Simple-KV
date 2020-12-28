fn main() {
    let s1 = String::from("hello");

    let len = calculate_length(&s1);
    if len {
        println!("The length of '{}' is {}.", s1, len);
    }
    println!("false, The length of '{}' is {}.", s1, len);
}

fn calculate_length(s: &String) -> bool {
    let a: String = s.clone();
    println!("'{}' ", a);
    s.len() == 2
}