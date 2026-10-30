#![forbid(unsafe_code)]
use liblivephoto_interface::{process, OperationResult, MAX_INPUT};
use serde_json::{json, Value};
use std::{
    fs::{self, File, OpenOptions},
    io::{self, Read, Seek, SeekFrom, Write},
    path::{Path, PathBuf},
};
const HELP:&str="LibLivePhoto CLI\n\nllp version\nllp parse|check|extract|metadata|extract-metadata|classify-metadata|agtm|xmp INPUT [options]\nllp build STILL --movie MOVIE --target google-jpeg|google-heif|google-avif|apple [options]\nllp conformance INPUT --options-json {PROFILE_JSON} [--tracks PRIMARY[,SECONDARY]]\nllp optimize|remove-tracks|track-summary MOVIE [options]\nllp import-tracks DEST --movie SOURCE --tracks IDS [options]\n\nOptions:\n  --out NEW_DIRECTORY   publish files and report without overwriting\n  --movie PATH          paired movie / import source\n  --parts               explicitly associate ordinary still and movie\n  --time VALUE/SCALE    exact integer photo/extraction time\n  --range OFFSET:LENGTH / --movie-range OFFSET:LENGTH\n  --strict              strict source parsing\n  --disable-3p / --enable-provider NAME (repeatable)\n  --target PROFILE / --pair-id ID\n  --sidecars / --allow-drop (explicit unmapped-resource policy)\n  --still / --video / --xmp (extract selected outputs; default all)\n  --resources IDS       extract explicit resource IDs\n  --blocks IDS          extract explicit metadata block indices\n  --tracks IDS          selected movie track IDs\n  --video-input         standalone-video AGTM input\n  --compact             explicitly discard unreferenced movie bytes\n  --dry-run             inspect build plan without generating files\n  --options-json JSON   structured protocol options (flags override)\n\nUse - as INPUT for bounded stdin. All source extents refer to the selected\nsnapshot; inputRanges in the report disclose enclosing file ranges. No operation\nuploads data, transcodes pixels, silently removes tracks or overwrites originals.\n";
fn main() {
    let code = match run() {
        Ok(code) => code,
        Err(e) => {
            eprintln!(
                "{}",
                json!({"success":false,"error":{"code":"cliError","message":e.to_string()}})
            );
            2
        }
    };
    std::process::exit(code);
}
fn next(
    args: &mut impl Iterator<Item = String>,
    flag: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    args.next()
        .ok_or_else(|| format!("{flag} requires a value").into())
}
fn range(s: &str) -> Result<(u64, u64), Box<dyn std::error::Error>> {
    let (offset, len) = s.split_once(':').ok_or("range requires OFFSET:LENGTH")?;
    let offset = offset.parse::<u64>()?;
    let len = len.parse::<u64>()?;
    if len == 0 || offset.checked_add(len).is_none() {
        return Err("empty or overflowing range".into());
    }
    Ok((offset, len))
}
fn read(
    path: &str,
    range: Option<(u64, u64)>,
    budget: usize,
) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    if path == "-" {
        if range.is_some() {
            return Err("stdin ranges are not supported".into());
        }
        let mut data = Vec::new();
        io::stdin()
            .lock()
            .take(budget as u64 + 1)
            .read_to_end(&mut data)?;
        if data.len() > budget {
            return Err("input byte budget exceeded".into());
        }
        return Ok(data);
    }
    let mut file = File::open(path)?;
    let length = file.metadata()?.len();
    if !file.metadata()?.is_file() {
        return Err("regular input file required".into());
    }
    let (offset, len) = range.unwrap_or((0, length));
    if len > budget as u64 || offset.checked_add(len).is_none_or(|n| n > length) {
        return Err("input range/budget exceeded".into());
    }
    file.seek(SeekFrom::Start(offset))?;
    let mut data = Vec::new();
    file.take(len).read_to_end(&mut data)?;
    if data.len() as u64 != len {
        return Err("input changed during snapshot".into());
    }
    Ok(data)
}
fn run() -> Result<i32, Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let Some(action) = args.next() else {
        print!("{HELP}");
        return Ok(0);
    };
    if matches!(action.as_str(), "--help" | "-h" | "help") {
        print!("{HELP}");
        return Ok(0);
    }
    let action = if action == "--version" {
        "version".into()
    } else {
        action
    };
    let mut request = json!({"action":action});
    let mut input = None;
    let mut movie = None;
    let mut out = None;
    let mut source_range = None;
    let mut movie_range = None;
    let mut selected = Vec::new();
    let mut providers = Vec::new();
    let mut flags = Vec::new();
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--out" => out = Some(PathBuf::from(next(&mut args, &arg)?)),
            "--movie" => movie = Some(next(&mut args, &arg)?),
            "--range" => source_range = Some(range(&next(&mut args, &arg)?)?),
            "--movie-range" => movie_range = Some(range(&next(&mut args, &arg)?)?),
            "--parts" => flags.push(("inputKind", json!("parts"))),
            "--strict" => flags.push(("strict", json!(true))),
            "--disable-3p" => flags.push(("disableThirdParty", json!(true))),
            "--enable-provider" => providers.push(next(&mut args, &arg)?),
            "--target" => flags.push(("target", json!(next(&mut args, &arg)?))),
            "--pair-id" => flags.push(("pairId", json!(next(&mut args, &arg)?))),
            "--sidecars" => flags.push(("unmapped", json!("sidecars"))),
            "--allow-drop" => flags.push(("unmapped", json!("drop"))),
            "--video-input" => flags.push(("videoInput", json!(true))),
            "--compact" => flags.push(("compact", json!(true))),
            "--dry-run" => flags.push(("dryRun", json!(true))),
            "--still" => selected.push("still"),
            "--video" => selected.push("video"),
            "--xmp" => selected.push("xmp"),
            "--tracks" | "--resources" | "--blocks" => {
                let list = next(&mut args, &arg)?
                    .split(',')
                    .map(str::parse::<u32>)
                    .collect::<Result<Vec<_>, _>>()?;
                flags.push((
                    if arg == "--tracks" {
                        "trackIds"
                    } else if arg == "--blocks" {
                        "blockIds"
                    } else {
                        "resourceIds"
                    },
                    json!(list),
                ));
            }
            "--time" => {
                let value = next(&mut args, &arg)?;
                let (value, scale) = value.split_once('/').ok_or("time requires VALUE/SCALE")?;
                let n = value.parse::<i64>()?;
                let scale = scale.parse::<u32>()?;
                if scale == 0 {
                    return Err("zero timescale".into());
                }
                flags.push(("time", json!({"value":n.to_string(),"timescale":scale})));
            }
            "--options-json" => {
                let v: Value = serde_json::from_str(&next(&mut args, &arg)?)?;
                let object = v.as_object().ok_or("options JSON must be an object")?;
                for (k, v) in object {
                    if k == "action" && v != &request["action"] {
                        return Err("options cannot change action".into());
                    }
                    request[k] = v.clone();
                }
            }
            "-" => {
                if input.replace(arg).is_some() {
                    return Err("only one primary input is accepted".into());
                }
            }
            value if value.starts_with('-') => return Err(format!("unknown option {value}").into()),
            _ => {
                if input.replace(arg).is_some() {
                    return Err("only one primary input is accepted; use --movie".into());
                }
            }
        }
    }
    if flags.iter().filter(|(k, _)| *k == "unmapped").count() > 1 {
        return Err("choose either --sidecars or --allow-drop".into());
    }
    for (k, v) in flags {
        request[k] = v;
    }
    if !providers.is_empty() {
        request["enabledProviders"] = json!(providers);
    }
    if !selected.is_empty() {
        for key in ["still", "video", "xmp"] {
            request[key] = json!(selected.contains(&key));
        }
    }
    if action == "build" && request.get("inputKind").is_none() {
        request["inputKind"] = json!("parts");
    }
    let primary = if action == "version" {
        Vec::new()
    } else {
        read(
            input.as_deref().ok_or("input required")?,
            source_range,
            MAX_INPUT,
        )?
    };
    let movie = movie
        .map(|p| read(&p, movie_range, MAX_INPUT - primary.len()))
        .transpose()?
        .unwrap_or_default();
    let result = process(&request.to_string(), &primary, &movie);
    let failed = !result.success || (action == "check" && result.metadata["valid"] != true);
    let report = json!({"success":result.success,"metadata":result.metadata,"files":result.files.iter().map(|f|json!({"name":f.name,"mime":f.mime,"bytes":f.bytes.len().to_string(),"resourceId":f.resource_id})).collect::<Vec<_>>(),"inputRanges":[{"offset":source_range.map_or(0,|v|v.0).to_string(),"length":primary.len().to_string()},{"offset":movie_range.map_or(0,|v|v.0).to_string(),"length":movie.len().to_string()}]});
    if !failed {
        if let Some(dir) = out {
            publish(&dir, &result, &report)?;
        } else if !result.files.is_empty() {
            return Err(
                "--out NEW_DIRECTORY is required to publish generated/extracted files".into(),
            );
        }
    }
    println!("{}", serde_json::to_string_pretty(&report)?);
    Ok(if failed { 1 } else { 0 })
}
fn publish(
    dir: &Path,
    result: &OperationResult,
    report: &Value,
) -> Result<(), Box<dyn std::error::Error>> {
    publish_with(dir, result, report, |file, bytes| file.write_all(bytes))
}
fn publish_with(
    dir: &Path,
    result: &OperationResult,
    report: &Value,
    mut write: impl FnMut(&mut File, &[u8]) -> io::Result<()>,
) -> Result<(), Box<dyn std::error::Error>> {
    let report = serde_json::to_vec_pretty(report)?;
    let mut names = std::collections::BTreeSet::from(["report.json"]);
    for file in &result.files {
        let mut components = Path::new(&file.name).components();
        if !matches!(
            (components.next(), components.next()),
            (Some(std::path::Component::Normal(_)), None)
        ) || !names.insert(&file.name)
        {
            return Err("unsafe or duplicate generated output name".into());
        }
    }
    fs::create_dir(dir)?;
    let mut created = Vec::new();
    let outcome = (|| -> io::Result<()> {
        for (name, bytes) in result
            .files
            .iter()
            .map(|f| (f.name.as_str(), f.bytes.as_slice()))
            .chain(std::iter::once(("report.json", report.as_slice())))
        {
            let path = dir.join(name);
            let mut file = OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&path)?;
            created.push(path);
            write(&mut file, bytes)?;
            file.sync_all()?;
        }
        Ok(())
    })();
    if let Err(error) = outcome {
        let mut cleanup_errors = Vec::new();
        for path in created.iter().rev() {
            if let Err(e) = fs::remove_file(path) {
                cleanup_errors.push(format!("{}: {e}", path.display()));
            }
        }
        // Never recursively delete: another writer may have added its own file.
        if let Err(e) = fs::remove_dir(dir) {
            cleanup_errors.push(format!("{}: {e}", dir.display()));
        }
        return Err(if cleanup_errors.is_empty() {
            format!("output write failed; new outputs rolled back: {error}")
        } else {
            format!(
                "output write failed: {error}; incomplete rollback: {}",
                cleanup_errors.join("; ")
            )
        }
        .into());
    }
    Ok(())
}

#[cfg(test)]
mod publication_tests {
    use super::*;
    use liblivephoto_interface::OperationFile;
    use std::sync::atomic::{AtomicUsize, Ordering};
    static NEXT: AtomicUsize = AtomicUsize::new(0);
    fn path() -> PathBuf {
        std::env::temp_dir().join(format!(
            "llp-rollback-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ))
    }
    fn result() -> OperationResult {
        OperationResult {
            success: true,
            metadata: json!({}),
            files: ["first.bin", "second.bin"]
                .into_iter()
                .map(|name| OperationFile {
                    name: name.into(),
                    mime: "application/octet-stream".into(),
                    resource_id: None,
                    bytes: b"original encoded bytes".to_vec(),
                })
                .collect(),
        }
    }
    #[test]
    fn failed_second_write_removes_earlier_output_and_the_partial_file() {
        let dir = path();
        let mut calls = 0;
        let error = publish_with(&dir, &result(), &json!({}), |file, bytes| {
            calls += 1;
            if calls == 2 {
                file.write_all(&bytes[..3])?;
                return Err(io::Error::other("injected storage failure"));
            }
            file.write_all(bytes)
        })
        .unwrap_err();
        assert!(error.to_string().contains("rolled back"));
        assert!(!dir.exists());
    }
    #[test]
    fn rollback_preserves_unrelated_files_and_reports_cleanup_failure() {
        let dir = path();
        let error = publish_with(&dir, &result(), &json!({}), |_, _| {
            fs::write(dir.join("unrelated.txt"), b"keep")?;
            Err(io::Error::other("injected storage failure"))
        })
        .unwrap_err();
        assert!(error.to_string().contains("incomplete rollback"));
        assert_eq!(fs::read(dir.join("unrelated.txt")).unwrap(), b"keep");
        assert!(!dir.join("first.bin").exists());
        fs::remove_file(dir.join("unrelated.txt")).unwrap();
        fs::remove_dir(dir).unwrap();
    }
}
