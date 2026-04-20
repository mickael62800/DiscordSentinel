use super::*;

    use super::*;

    #[test]
    fn proto_to_flags_round_trip_all_true() {
        let p = proto::DetectionFlags { spam: true, insult: true, link: true, phishing: true };
        let f = proto_to_flags(p);
        assert!(f.spam && f.insult && f.link && f.phishing);
    }

    #[test]
    fn proto_to_flags_round_trip_mixed() {
        let p = proto::DetectionFlags { spam: true, insult: false, link: true, phishing: false };
        let f = proto_to_flags(p);
        assert!(f.spam);
        assert!(!f.insult);
        assert!(f.link);
        assert!(!f.phishing);
    }

    #[test]
    fn action_to_proto_all_variants() {
        assert_eq!(action_to_proto(Action::None), proto::Action::None as i32);
        assert_eq!(action_to_proto(Action::Warn), proto::Action::Warn as i32);
        assert_eq!(action_to_proto(Action::Delete), proto::Action::Delete as i32);
        assert_eq!(action_to_proto(Action::Mute), proto::Action::Mute as i32);
        assert_eq!(action_to_proto(Action::Ban), proto::Action::Ban as i32);
    }

    #[test]
    fn analysis_to_proto_full_mapping() {
        let a = MessageAnalysis {
            action: Action::Warn,
            reason: "spam".into(),
            score: 0.65,
            duration: Some(300),
        };
        let p = analysis_to_proto(a);
        assert_eq!(p.action, proto::Action::Warn as i32);
        assert_eq!(p.reason, "spam");
        assert!((p.score - 0.65).abs() < 1e-6);
        assert_eq!(p.duration, Some(300));
    }

    #[test]
    fn analysis_to_proto_no_action() {
        let a = MessageAnalysis {
            action: Action::None,
            reason: String::new(),
            score: 0.0,
            duration: None,
        };
        let p = analysis_to_proto(a);
        assert_eq!(p.action, proto::Action::None as i32);
        assert!(p.duration.is_none());
    }
