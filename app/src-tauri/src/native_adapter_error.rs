//! IPC errors for adapters that retain existing operation strings while
//! forwarding the canonical, bounded query error without losing its code.
use protocol::control_plane_query::QueryError;
use serde::Serialize;

#[derive(Debug, PartialEq, Eq, Serialize)]
#[serde(untagged)]
pub(crate) enum NativeAdapterError {
    Operation(String),
    Query(QueryError),
}

impl From<String> for NativeAdapterError {
    fn from(message: String) -> Self {
        Self::Operation(message)
    }
}

impl From<&str> for NativeAdapterError {
    fn from(message: &str) -> Self {
        Self::Operation(message.into())
    }
}

impl From<QueryError> for NativeAdapterError {
    fn from(error: QueryError) -> Self {
        Self::Query(error)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use protocol::control_plane_query::QueryErrorCode;

    #[test]
    fn native_adapter_error_preserves_every_canonical_query_wire() {
        use QueryErrorCode::*;
        for code in [
            InvalidRequest,
            UnsupportedProtocolVersion,
            UnknownOperation,
            Unauthorized,
            Forbidden,
            NotFound,
            Unavailable,
            SchemaMismatch,
            CursorInvalid,
            CursorStale,
            SnapshotRequired,
            EventGap,
            RateLimited,
            Overloaded,
            Internal,
        ] {
            let expected = QueryError::from_code(code);
            let wire = serde_json::to_value(NativeAdapterError::from(expected.clone())).unwrap();
            assert_eq!(wire, serde_json::to_value(&expected).unwrap());
            assert_eq!(
                serde_json::from_value::<QueryError>(wire).unwrap(),
                expected
            );
        }
    }

    #[test]
    fn native_adapter_error_keeps_operation_strings_and_query_flags_distinct() {
        for error in [
            NativeAdapterError::from("project_file_request_invalid"),
            NativeAdapterError::from("project_file_request_invalid".to_string()),
        ] {
            assert_eq!(
                serde_json::to_value(error).unwrap(),
                "project_file_request_invalid"
            );
        }
        assert_eq!(
            serde_json::to_value(NativeAdapterError::from(QueryError::from_code(
                QueryErrorCode::Overloaded
            )))
            .unwrap(),
            serde_json::json!({"code":"overloaded", "message":"query service overloaded",
                "retryable":true, "resnapshot_required":false})
        );
    }
}
