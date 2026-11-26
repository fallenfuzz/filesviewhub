// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
  let args: Vec<String> = std::env::args().skip(1).collect();
  if args.first().map(String::as_str) == Some(filesviewhub_lib::PDF_FLAG) {
    std::process::exit(filesviewhub_lib::pdf_helper(&args[1..]));
  }
  filesviewhub_lib::run();
}
