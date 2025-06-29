use std::fs;

pub fn extract_cmdline(pid: &i32) -> Result<String, std::io::Error> {
    let path = format!("/proc/{pid}/cmdline");

    let data = fs::read(&path)?;
    let joined = data
        .split(|&b| b == 0)
        .filter(|part| !part.is_empty())
        .map(|part| String::from_utf8_lossy(part))
        .collect::<Vec<_>>()
        .join(" ");

    Ok(joined)
}
