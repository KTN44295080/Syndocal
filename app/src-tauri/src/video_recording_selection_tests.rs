use super::*;

#[test]
fn recording_encoder_errors_are_static_localization_source_keys() {
    assert_eq!(
        actionable_error("recording.encoderUnavailable: variable GPU driver detail".into()),
        ENCODER_ERROR
    );
    assert_eq!(
        actionable_error("recording.encoderLicenseRejected: variable build flags".into()),
        LICENSE_ERROR
    );
    assert_eq!(
        actionable_error("recording.encoderExecutableInvalid: C:/variable/path".into()),
        EXECUTABLE_ERROR
    );
}

#[test]
fn recording_encoder_chain_and_mapping_are_bounded_to_lgpl_candidates() {
    assert_eq!(
        CHAIN,
        [
            "h264_nvenc",
            "h264_amf",
            "h264_qsv",
            "h264_mf",
            "libopenh264"
        ]
    );
    let mappings: [&[&str]; 5] = [
        &["-preset", "p2", "-rc", "vbr", "-cq", "23", "-b:v", "0"],
        &[
            "-quality", "speed", "-rc", "cqp", "-qp_i", "23", "-qp_p", "23",
        ],
        &["-preset", "veryfast", "-global_quality", "23"],
        &[
            "-rate_control",
            "quality",
            "-quality",
            "75",
            "-scenario",
            "archive",
        ],
        &[
            "-rc_mode",
            "bitrate",
            "-b:v",
            "8M",
            "-allow_skip_frames",
            "0",
        ],
    ];
    for (encoder, expected) in CHAIN.into_iter().zip(mappings) {
        let mut command = Command::new("unused");
        options(&mut command, encoder, 30).unwrap();
        let args: Vec<_> = command
            .get_args()
            .map(|s| s.to_string_lossy().into_owned())
            .collect();
        let mut all = vec!["-c:v", encoder, "-pix_fmt", "yuv420p", "-g", "60"];
        all.extend_from_slice(expected);
        assert_eq!(args, all, "{encoder}");
    }
    assert!(options(&mut Command::new("unused"), "libx264", 30).is_err());
}

#[test]
fn recording_encoder_rejects_gpl_nonfree_and_unverifiable_license() {
    let license = "GNU Lesser General Public License";
    let version = "ffmpeg version 8.1 --disable-gpl --disable-nonfree";
    assert!(validate_license(license, version).is_ok());
    for bad in [
        "--enable-gpl",
        "--enable-nonfree",
        "nonfree and unredistributable",
    ] {
        assert!(validate_license(license, &format!("{version} {bad}")).is_err());
        assert!(validate_license(&format!("{license} {bad}"), version).is_err());
    }
    assert!(validate_license("GNU General Public License", version).is_err());
    assert!(validate_license(&format!("{license}\nGNU General Public License"), version).is_err());
    assert!(validate_license("", version).is_err());
    assert!(validate_license(license, "").is_err());
}

#[test]
fn recording_encoder_resolution_prioritizes_bundle_and_identity_changes_with_content() {
    let root = env::temp_dir().join(format!(
        "syndocal-resolution-{}-{}",
        std::process::id(),
        TEMP_ID.fetch_add(1, Ordering::Relaxed)
    ));
    fs::create_dir_all(&root).unwrap();
    let bundled = root.join("bundled.exe");
    let development = root.join("development.exe");
    fs::write(&bundled, b"bundle").unwrap();
    fs::write(&development, b"development").unwrap();
    assert_eq!(
        resolve_from(&bundled, Some(development.as_os_str()), None).unwrap(),
        bundled
    );
    let before = identity(&bundled).unwrap();
    fs::write(&bundled, b"changed").unwrap();
    assert_ne!(before, identity(&bundled).unwrap());
    fs::remove_file(&bundled).unwrap();
    assert_eq!(
        resolve_from(&bundled, Some(development.as_os_str()), None).unwrap(),
        development
    );
    assert!(resolve_from(&bundled, None, Some(OsStr::new(""))).is_err());
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn recording_encoder_preflight_timeout_terminates_and_reaps_real_child() {
    #[cfg(windows)]
    let mut command = {
        let mut c = hidden_command("cmd.exe");
        c.args(["/D", "/C", "ping -n 30 127.0.0.1 >NUL"]);
        c
    };
    #[cfg(not(windows))]
    let mut command = {
        let mut c = Command::new("sleep");
        c.arg("30");
        c
    };
    command
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    let start = Instant::now();
    let error = bounded(&mut command, Duration::from_millis(100)).unwrap_err();
    assert!(error.contains("terminated/reaped"), "{error}");
    assert!(start.elapsed() < Duration::from_secs(5));
}

#[test]
fn recording_encoder_preparation_cancelled_before_spawn() {
    let stop = AtomicBool::new(true);
    let mut command = Command::new("this-command-must-never-be-spawned");
    assert!(bounded_inner(&mut command, PROBE_TIMEOUT, Some(&stop))
        .unwrap_err()
        .contains("cancelled"));
    assert!(select_inner(Path::new("missing-ffmpeg"), Some(&stop))
        .unwrap_err()
        .contains("cancelled"));
}
