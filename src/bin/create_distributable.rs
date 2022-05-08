use fs_extra;
use std::fs;
use std::process::Command;

fn main() {
  let build_output = Command::new("cargo")
    .args(["build", "--release"])
    .stdout(std::process::Stdio::inherit())
    .stderr(std::process::Stdio::inherit())
    .output()
    .unwrap();

  if !build_output.status.success() {
    // stdout + stderr are shared, no need to display anything
    return;
  }

  // areas
  fs_extra::dir::create_all("dist/areas", true).unwrap();

  fs_extra::dir::copy("areas", "dist", &fs_extra::dir::CopyOptions::default()).unwrap();

  // assets
  fs_extra::dir::copy("assets", "dist", &fs_extra::dir::CopyOptions::default()).unwrap();

  // scripts
  fs_extra::dir::create_all("dist/scripts", true).unwrap();

  fs_extra::dir::copy(
    "scripts/libs",
    "dist/scripts",
    &fs_extra::dir::CopyOptions::default(),
  )
  .unwrap();

  // licenses
  let cargo_about_output = Command::new("cargo")
    .args(["about", "generate", "about.hbs"])
    .stderr(std::process::Stdio::inherit())
    .output()
    .unwrap();

  if !cargo_about_output.status.success() {
    // stdout + stderr are shared, no need to display anything
    return;
  }

  let _ = fs::write("dist/third_party_licenses.html", &cargo_about_output.stdout);

  let exe_name = env!("CARGO_PKG_NAME").replace("-", "_");

  // windows exe
  let _ = fs::copy(
    format!("target/release/{}.exe", exe_name),
    format!("dist/{}.exe", exe_name),
  );

  // linux exe
  let _ = fs::copy(
    format!("target/release/{}", exe_name),
    format!("dist/{}", exe_name),
  );
}
