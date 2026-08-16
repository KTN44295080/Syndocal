from pathlib import Path


def replace_exact(path: Path, old: str, new: str, label: str) -> None:
    text = path.read_text(encoding="utf-8")
    count = text.count(old)
    if count != 1:
        raise RuntimeError(f"{label}: expected exactly one match, found {count}")
    path.write_text(text.replace(old, new, 1), encoding="utf-8")


protocol = Path("crates/protocol/src/control_plane_command.rs")

# Receipt lookup may skip consent re-validation only for a byte-identical retry.
# Bind request_id and the opaque consent token into the shape hash; the raw token
# is never stored in audit/receipt state, only its SHA-256-derived request shape.
replace_exact(
    protocol,
    '''        let mut bytes = OUTPUT_CONTROL_SHAPE_DOMAIN_V1.to_vec();
        append_ascii(&mut bytes, &self.operation_id)
            .map_err(|_| OutputControlValidationErrorV1::UnexpectedOperationId)?;
        self.expected_fence.append_canonical_bytes(&mut bytes)?;
        self.action.append_canonical_bytes(&mut bytes)?;
        Ok(bytes)
''',
    '''        let mut bytes = OUTPUT_CONTROL_SHAPE_DOMAIN_V1.to_vec();
        append_ascii(&mut bytes, &self.operation_id)
            .map_err(|_| OutputControlValidationErrorV1::UnexpectedOperationId)?;
        append_u64(&mut bytes, self.request_id);
        self.expected_fence.append_canonical_bytes(&mut bytes)?;
        append_ascii(&mut bytes, &self.consent_token)
            .map_err(|_| OutputControlValidationErrorV1::InvalidOpaqueId)?;
        self.action.append_canonical_bytes(&mut bytes)?;
        Ok(bytes)
''',
    "bind output request identity and consent token into canonical shape",
)

replace_exact(
    protocol,
    '''        assert_eq!(
            request.canonical_shape_bytes().unwrap(),
            internal.canonical_shape_bytes().unwrap()
        );
        assert_eq!(
            request.argument_fingerprint_bytes().unwrap(),
            internal.argument_fingerprint_bytes().unwrap()
        );
''',
    '''        assert_eq!(
            request.canonical_shape_bytes().unwrap(),
            internal.canonical_shape_bytes().unwrap()
        );
        assert_eq!(
            request.argument_fingerprint_bytes().unwrap(),
            internal.argument_fingerprint_bytes().unwrap()
        );
        let mut different_token = request.clone();
        different_token.consent_token = "AQEBAQEBAQEBAQEBAQEBAQ".to_string();
        different_token.validate().unwrap();
        assert_ne!(
            request.canonical_shape_bytes().unwrap(),
            different_token.canonical_shape_bytes().unwrap(),
            "same request ID with a different consent token is not an exact retry"
        );
        assert_eq!(
            request.argument_fingerprint_bytes().unwrap(),
            different_token.argument_fingerprint_bytes().unwrap(),
            "consent token is request identity, not the target argument fingerprint"
        );
''',
    "strict wire same-ID different-token shape test",
)
