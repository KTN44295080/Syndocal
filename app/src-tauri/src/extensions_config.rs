pub fn decode(text: &str) -> Result<bool, String> {
    if text.len() > 4096 {
        return Err("Extension settings exceed their size limit".into());
    }
    let lines: Vec<_> = text
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect();
    if lines.len() != 3 || lines[0] != "[SyndocalExtensions]" || lines[1] != "Schema=1" {
        return Err("Unsupported extension settings; preserve the file and install a compatible Syndocal version".into());
    }
    match lines[2] {
        "DJLink=0" => Ok(false),
        "DJLink=1" => Ok(true),
        _ => Err("Invalid DJ Link extension choice".into()),
    }
}
