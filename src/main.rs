#![windows_subsystem = "windows"]

mod app;
mod localization;
mod netpage;
mod options;
mod pages;
mod perfpage;
mod procpage;
#[allow(dead_code)]
mod resource;
mod taskpage;
mod userpage;
mod winutil;

fn main() {
    std::process::exit(app::run());
}
