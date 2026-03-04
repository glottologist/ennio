use sha2::{Digest, Sha256};
use subtle::ConstantTimeEq;
use tonic::{Request, Status};

pub fn make_interceptor(
    expected_token: String,
) -> impl Fn(Request<()>) -> Result<Request<()>, Status> + Clone {
    move |req: Request<()>| {
        let auth_value = req
            .metadata()
            .get("authorization")
            .and_then(|v| v.to_str().ok());
        match auth_value {
            Some(v) => {
                let token = v.strip_prefix("Bearer ").ok_or_else(|| {
                    Status::unauthenticated("authorization header must use Bearer scheme")
                })?;
                let hash_token = Sha256::digest(token.as_bytes());
                let hash_expected = Sha256::digest(expected_token.as_bytes());
                if hash_token.ct_eq(&hash_expected).into() {
                    Ok(req)
                } else {
                    Err(Status::unauthenticated("invalid token"))
                }
            }
            None => Err(Status::unauthenticated("missing authorization header")),
        }
    }
}

#[cfg(test)]
mod tests {
    use rstest::rstest;
    use tonic::metadata::MetadataValue;

    use super::*;

    fn request_with_header(key: &'static str, value: &str) -> Request<()> {
        let mut req = Request::new(());
        req.metadata_mut()
            .insert(key, MetadataValue::try_from(value).unwrap());
        req
    }

    #[test]
    fn accepts_valid_token() {
        let interceptor = make_interceptor("test-secret-123".to_string());
        let req = request_with_header("authorization", "Bearer test-secret-123");
        assert!(interceptor(req).is_ok());
    }

    #[test]
    fn rejects_wrong_token() {
        let interceptor = make_interceptor("correct-token".to_string());
        let req = request_with_header("authorization", "Bearer wrong-token-val");
        let err = interceptor(req).unwrap_err();
        assert_eq!(err.code(), tonic::Code::Unauthenticated);
    }

    #[test]
    fn rejects_missing_header() {
        let interceptor = make_interceptor("some-token".to_string());
        let req = Request::new(());
        let err = interceptor(req).unwrap_err();
        assert_eq!(err.code(), tonic::Code::Unauthenticated);
    }

    #[rstest]
    #[case("Basic dXNlcjpwYXNz")]
    #[case("Token abc123")]
    #[case("bearer lowercase")]
    fn rejects_non_bearer_scheme(#[case] auth_value: &str) {
        let interceptor = make_interceptor("some-token".to_string());
        let req = request_with_header("authorization", auth_value);
        let err = interceptor(req).unwrap_err();
        assert_eq!(err.code(), tonic::Code::Unauthenticated);
    }

    #[test]
    fn rejects_empty_bearer_token() {
        let interceptor = make_interceptor("nonempty".to_string());
        let req = request_with_header("authorization", "Bearer ");
        let err = interceptor(req).unwrap_err();
        assert_eq!(err.code(), tonic::Code::Unauthenticated);
    }
}
