use std::process::exit;

fn main() {
    let result = flashr::run();
    match result {
        Ok((total_correct, total)) => {
            println!(
                "You got {total_correct} correct out of {total} ({:.2}%)",
                if total == 0 {
                    0.0
                } else {
                    (total_correct as f64 / total as f64) * 100.0
                }
            );
            if total_correct == total && total > 0 {
                println!("Well done!");
            }
        }
        Err(err) => {
            eprintln!("Error: {err}");
            exit(1);
        }
    }
}
