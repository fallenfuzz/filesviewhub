/// The commit count of the private repository the app was split from, at the split (build 476).
const EARLIER_BUILDS: u32 = 476;

fn main() {
    // The build number is the count of commits, so it rises with every committed change, plus the
    // builds made before this repository started (EARLIER_BUILDS), so it never goes back. The Linux
    // build folder has no .git, so scripts/linux.sh hands the number over in FVH_BUILD.
    println!("cargo:rerun-if-env-changed=FVH_BUILD");
    let build = std::env::var("FVH_BUILD").ok().filter(|b| !b.is_empty()).unwrap_or_else(|| {
        // The checkout's .git, wherever the app's folder sits in it.
        if let Some(git) = std::process::Command::new("git").args(["rev-parse", "--absolute-git-dir"]).output().ok()
            .filter(|o| o.status.success()).map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        {
            println!("cargo:rerun-if-changed={git}/HEAD");
            println!("cargo:rerun-if-changed={git}/refs/heads");
        }
        std::process::Command::new("git")
            .args(["rev-list", "--count", "HEAD"])
            .output()
            .ok()
            .filter(|o| o.status.success())
            .and_then(|o| String::from_utf8_lossy(&o.stdout).trim().parse::<u32>().ok())
            .map_or_else(|| "0".into(), |n| (n + EARLIER_BUILDS).to_string())
    });
    println!("cargo:rustc-env=FVH_BUILD={build}");
    tauri_build::build()
}
