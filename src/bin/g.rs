use proconio::input;

fn main() {
    input! {
        n: usize,
        a: [i64; n],
    }
    let ans: i64 = a.iter().sum();
    println!("{ans}");
}
