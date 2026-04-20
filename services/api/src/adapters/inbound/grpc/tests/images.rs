use super::*;

    use super::*;

    #[test]
    fn action_to_proto_all_variants() {
        assert_eq!(action_to_proto(Action::None), proto::Action::None as i32);
        assert_eq!(action_to_proto(Action::Warn), proto::Action::Warn as i32);
        assert_eq!(action_to_proto(Action::Delete), proto::Action::Delete as i32);
        assert_eq!(action_to_proto(Action::Mute), proto::Action::Mute as i32);
        assert_eq!(action_to_proto(Action::Ban), proto::Action::Ban as i32);
    }

    #[test]
    fn classification_to_proto_mapping() {
        let c = ImageClassification { label: "weapon".into(), confidence: 0.92 };
        let p = classification_to_proto(c);
        assert_eq!(p.label, "weapon");
        assert!((p.confidence - 0.92).abs() < 1e-6);
    }

    #[test]
    fn analysis_to_proto_full_mapping() {
        let a = ImageAnalysis {
            action: Action::Delete,
            reason: "violence detectee".into(),
            score: 0.87,
            duration: Some(150),
            classifications: vec![
                ImageClassification { label: "violence".into(), confidence: 0.87 },
                ImageClassification { label: "neutral".into(), confidence: 0.13 },
            ],
        };
        let p = analysis_to_proto(a);
        assert_eq!(p.action, proto::Action::Delete as i32);
        assert_eq!(p.reason, "violence detectee");
        assert!((p.score - 0.87).abs() < 1e-6);
        assert_eq!(p.duration, Some(150));
        assert_eq!(p.classifications.len(), 2);
        assert_eq!(p.classifications[0].label, "violence");
    }

    #[test]
    fn analysis_to_proto_no_action_no_classifications() {
        let a = ImageAnalysis {
            action: Action::None,
            reason: "ok".into(),
            score: 0.0,
            duration: None,
            classifications: vec![],
        };
        let p = analysis_to_proto(a);
        assert_eq!(p.action, proto::Action::None as i32);
        assert!(p.classifications.is_empty());
        assert!(p.duration.is_none());
    }
