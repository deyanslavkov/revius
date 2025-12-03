use colored::Colorize;
use std::path::Path;

pub fn print_init_success(root: &Path) {
    println!("{}", format!("Initialized empty Revius repository in {}/.rvs/", root.display()).green());
}

pub fn print_info(msg: &str) {
    println!("{}", msg.blue());
}

pub fn print_warn(msg: &str) {
    println!("{}", msg.yellow());
}
