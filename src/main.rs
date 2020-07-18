#![warn(clippy::all)]

use std::collections::HashSet;
use std::env;
use std::fs;
use std::io;
use std::path::Path;
use std::time::Instant;

use anyhow::{Context, Result};
use flate2::read::GzDecoder;

fn main() -> Result<()> {
    assert_eq!(env::args().len(), 3);
    let mut i = env::args_os();
    i.next();
    let tar_name = i.next().unwrap();
    let output_name = i.next().unwrap();
    let output = Path::new(&output_name);
    if output.exists() {
        println!("cleaning '{}'", output.to_string_lossy());
        remove_dir_all::remove_dir_all(&output)?;
    }
    fs::create_dir(&output)?;
    println!(
        "unpacking {} to '{}'",
        tar_name.to_string_lossy(),
        output.to_string_lossy()
    );
    let start = Instant::now();
    let tgz = io::BufReader::with_capacity(8 * 1024 * 1024, fs::File::open(tar_name)?);
    let tar = GzDecoder::new(tgz);
    let mut archive = tar::Archive::new(tar);
    let entries = archive.entries()?;
    let pool = threadpool::Builder::new()
        .thread_name("CloseHandle".into())
        .build();
    let mut made_parents = HashSet::new();
    for entry in entries {
        let mut entry = entry?;
        // save a syscall per file
        entry.set_preserve_mtime(false);
        // Note that this is vulnerable to .. escaping attacks and the like.
        // checking components for their kind etc would be important in
        // production code. see tar/entry.rs.html#174
        let full_path = output.join(entry.path()?);
        // println!("{}", full_path.to_string_lossy());
        if let Some(parent) = full_path.parent() {
            if !made_parents.contains(parent) {
                made_parents.insert(parent.to_owned());
                // Many tars are not wellformed tars - that is a/b/c will not be preceeded by a/b. npm 6.0.0 for
                // instance contains
                //
                // package/.github/issue_template.md
                // package/bin/node-gyp-bin/node-gyp
                //
                // with no directory nodes in between. We are somewhat wasteful with stats as a result: more
                // complicated code could cache all the missing nodes, but be less understandable.
                ::std::fs::create_dir_all(&parent)?;
            }
        };
        entry
            .unpack(&full_path)
            .map(|fd| {
                pool.execute(move || {
                    drop(fd);
                });
            })
            .with_context(|| format!("Failed to unpack {}", full_path.to_string_lossy()))?;
    }
    let duration = start.elapsed();
    println!("Extraction took {:?}", duration);
    Ok(())
}
