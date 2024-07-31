use std::process::exit;

fn main() {
    let result = flashr::run();
    match result {
        Ok(correct_incorrect) => {
            if let Some((total_correct, total)) = correct_incorrect {
                let percent_correct = if total == 0 {
                    0.0
                } else {
                    (total_correct as f64 / total as f64) * 100.0
                };

                println!(
                    "You got {total_correct} correct out of {total} ({:.2}%)",
                    if total == 0 { 0.0 } else { percent_correct }
                );

                if total >= 10 {
                    if percent_correct == 100.0 {
                        if total >= 1000 {
                            println!("🌌🌟🚀 Out of this world! 🚀🌟🌌")
                        } else if total >= 100 {
                            println!("🚀🌌 Spectacular! 🌌🚀")
                        } else {
                            println!("🌟 Perfect! 🌟");
                        }
                    } else if percent_correct >= 0.9 {
                        println!("🥇 Excellent! 🥇");
                    } else if percent_correct >= 0.8 {
                        println!("🥈 Well done! 🥈");
                    } else if percent_correct >= 0.7 {
                        println!("🥉 Nice! 🥉");
                    } else {
                        println!("Keep up the practice!");
                    }
                }
            }
        }
        Err(err) => {
            eprintln!("Error: {err}");
            exit(1);
        }
    }
}
