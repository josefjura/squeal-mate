pub fn round_up_division(first: usize, second: usize) -> usize {
    (first + second - 1) / second
}

pub fn get_max_length<'a>(lines: &'a [(&'a str, &'a str)]) -> u16 {
    lines
        .iter()
        .map(|f| f.0.len() + f.1.len())
        .max()
        .unwrap_or(10) as u16
}
