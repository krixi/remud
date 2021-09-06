pub fn word_list(mut words: Vec<String>) -> String {
    if words.is_empty() {
        String::new()
    } else if words.len() == 1 {
        words.pop().unwrap()
    } else if words.len() == 2 {
        words.join(" and ")
    } else {
        let last = words.pop().unwrap();
        let list = words.join(", ");
        format!("{}, and {}", list, last)
    }
}
