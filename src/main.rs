use std::process::exit;

use flashr::Progress;

fn main() {
    let result = flashr::run();
    match result {
        Ok(progress) => {
            if let Some(progress) = progress {
                let (_, percent) = progress.ratio_percent();
                let Progress { correct, total } = progress;

                println!("You got {correct} correct out of {total} ({percent:.2}%)");

                if total >= 10 {
                    if percent == 100.0 {
                        if total >= 1000 {
                            println!("🌌🌟🚀 Out of this world! 🚀🌟🌌")
                        } else if total >= 100 {
                            println!("🚀🌌 Spectacular! 🌌🚀")
                        } else {
                            println!("🌟 Perfect! 🌟");
                        }
                    } else if percent >= 0.9 {
                        println!("🥇 Excellent! 🥇");
                    } else if percent >= 0.8 {
                        println!("🥈 Well done! 🥈");
                    } else if percent >= 0.7 {
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
