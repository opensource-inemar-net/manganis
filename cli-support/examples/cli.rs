use manganis_cli_support::{AssetManifestExt, ManganisSupportGuard};
use manganis_common::{AssetManifest, Config};
use std::path::PathBuf;
use std::process::{ChildStdout, Stdio};

use cargo_metadata::Message;

// return the location of the executable generated by cargo
fn get_executable_location(cargo_output: std::io::BufReader<ChildStdout>) -> PathBuf {
    let executable = cargo_metadata::Message::parse_stream(cargo_output).find_map(|x| {
        if let Ok(Message::CompilerArtifact(artifact)) = x {
            artifact.executable
        } else {
            None
        }
    });
    let executable = executable.expect("Failed to find the output binary path. This may happen if you build a library instead of an application");

    executable.into_std_path_buf()
}

fn main() {
    use std::process::Command;

    // This is the location where the assets will be copied to in the filesystem
    let assets_file_location = "./assets";
    // This is the location where the assets will be served from
    let assets_serve_location = "/assets";

    // First set any settings you need for the build
    Config::default()
        .with_assets_serve_location(assets_serve_location)
        .save();

    // Next, tell manganis that you support assets
    let _guard = ManganisSupportGuard::default();

    // Then build your application
    let mut command = Command::new("cargo")
        .args([
            "build",
            "--message-format=json-render-diagnostics",
            "--release",
        ])
        .stdout(Stdio::piped())
        // On macOS we need to set lto to thin to make sure assets link correctly
        .env("RUSTFLAGS", "-C embed-bitcode=yes -C lto=true")
        .spawn()
        .unwrap();

    let reader = std::io::BufReader::new(command.stdout.take().unwrap());
    let path = get_executable_location(reader);

    // Then collect the assets
    let assets = AssetManifest::load(&path);

    // Remove the old assets
    let _ = std::fs::remove_dir_all(assets_file_location);

    // And copy the static assets to the public directory
    assets.copy_static_assets_to(assets_file_location).unwrap();

    // Then collect the tailwind CSS
    let css = assets.collect_tailwind_css(true, &mut Vec::new());

    // And write the CSS to the public directory
    std::fs::write(format!("{}/tailwind.css", assets_file_location), css).unwrap();
}
