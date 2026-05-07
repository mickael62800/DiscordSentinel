use super::*;

    fn req_with_auth(value: Option<&str>) -> Request<()> {
        let mut req = Request::new(());
        if let Some(v) = value {
            req.metadata_mut()
                .insert("authorization", v.parse().unwrap());
        }
        req
    }

    #[test]
    fn empty_api_key_disables_auth_and_passes_through() {
        let interceptor = build_auth_interceptor(String::new());
        // Sans header
        assert!(interceptor(req_with_auth(None)).is_ok());
        // Avec header arbitraire
        assert!(interceptor(req_with_auth(Some("Bearer whatever"))).is_ok());
    }

    #[test]
    fn correct_bearer_token_is_accepted() {
        let interceptor = build_auth_interceptor("secret123".to_string());
        let req = req_with_auth(Some("Bearer secret123"));
        assert!(interceptor(req).is_ok());
    }

    #[test]
    fn missing_token_is_unauthenticated() {
        let interceptor = build_auth_interceptor("secret123".to_string());
        let err = interceptor(req_with_auth(None)).unwrap_err();
        assert_eq!(err.code(), tonic::Code::Unauthenticated);
    }

    #[test]
    fn wrong_token_is_unauthenticated() {
        let interceptor = build_auth_interceptor("secret123".to_string());
        let err = interceptor(req_with_auth(Some("Bearer wrong"))).unwrap_err();
        assert_eq!(err.code(), tonic::Code::Unauthenticated);
    }

    #[test]
    fn token_without_bearer_prefix_is_unauthenticated() {
        let interceptor = build_auth_interceptor("secret123".to_string());
        let err = interceptor(req_with_auth(Some("secret123"))).unwrap_err();
        assert_eq!(err.code(), tonic::Code::Unauthenticated);
    }

    #[test]
    fn invalid_api_key_chars_disable_auth() {
        // Caracteres de controle (NUL/newline) -> parse() echoue -> auth
        // desactivee (mode fallback safe).
        let interceptor = build_auth_interceptor("bad\nkey\0".to_string());
        assert!(interceptor(req_with_auth(None)).is_ok());
    }
