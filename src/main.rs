use phi_specials::export;
use phi_unpackd::publish;
use std::path::Path;

const USAGE: &str = "usage:
  phi-specials extract <apk> <output dir>
  phi-specials publish <output dir> [--dry-run]   (needs the daemon's Cloudflare settings, e.g. from unpackd.env)";

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let result = match args
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>()
        .as_slice()
    {
        ["extract", apk, out] => extract(Path::new(apk), Path::new(out)),
        ["publish", out] => publish(Path::new(out), false),
        ["publish", out, "--dry-run"] => publish(Path::new(out), true),
        _ => {
            eprintln!("{USAGE}");
            std::process::exit(2);
        }
    };
    if let Err(err) = result {
        eprintln!("phi-specials: {err}");
        std::process::exit(1);
    }
}

fn extract(apk: &Path, out: &Path) -> Result<(), String> {
    let report = export::extract(apk, out)?;
    for warning in &report.warnings {
        eprintln!("warning: {warning}");
    }
    for file in &report.files {
        println!("{file}");
    }
    Ok(())
}

/// The files first, then the manifests that name them; R2 objects with the same size and etag are left alone
fn publish(out: &Path, dry_run: bool) -> Result<(), String> {
    let (files, manifests) = export::uploads(out)?;
    let report = publish::upload(&[&files, &manifests], &out.join("published.json"), dry_run)?;
    for (key, sent) in &report {
        let what = match (sent, dry_run) {
            (true, true) => "would upload",
            (true, false) => "uploaded",
            (false, _) => "unchanged",
        };
        println!("{what}\t{key}");
    }
    Ok(())
}
