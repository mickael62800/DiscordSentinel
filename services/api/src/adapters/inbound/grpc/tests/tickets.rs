use super::*;

    use super::*;
    use chrono::TimeZone;
    use uuid::Uuid;

    fn ts() -> chrono::DateTime<chrono::Utc> {
        chrono::Utc.with_ymd_and_hms(2026, 1, 15, 12, 0, 0).unwrap()
    }

    fn sample_ticket() -> Ticket {
        Ticket {
            id: Uuid::nil(),
            title: "Bug critical".into(),
            status: "open".into(),
            priority: "high".into(),
            author_id: "u1".into(),
            author_name: "Joe".into(),
            assigned_to: Some("mod1".into()),
            server: "main".into(),
            category: "bug".into(),
            ticket_type: "support".into(),
            channel_id: Some("c1".into()),
            voice_channel_id: None,
            invited_user_id: None,
            created_at: ts(),
            updated_at: ts(),
            messages_count: 5,
        }
    }

    #[test]
    fn ticket_to_proto_full_mapping() {
        let p = ticket_to_proto(sample_ticket());
        assert_eq!(p.title, "Bug critical");
        assert_eq!(p.status, "open");
        assert_eq!(p.priority, "high");
        assert_eq!(p.assigned_to.as_deref(), Some("mod1"));
        assert_eq!(p.channel_id.as_deref(), Some("c1"));
        assert!(p.voice_channel_id.is_none());
        assert_eq!(p.messages_count, 5);
        assert_eq!(p.created_at, ts().to_rfc3339());
    }

    #[test]
    fn ticket_to_proto_unassigned() {
        let mut t = sample_ticket();
        t.assigned_to = None;
        t.invited_user_id = None;
        let p = ticket_to_proto(t);
        assert!(p.assigned_to.is_none());
        assert!(p.invited_user_id.is_none());
    }

    #[test]
    fn ticket_message_to_proto_mapping() {
        let m = TicketMessage {
            id: Uuid::nil(),
            ticket_id: Uuid::nil(),
            author_name: "Joe".into(),
            author_role: "user".into(),
            content: "Help!".into(),
            created_at: ts(),
        };
        let p = ticket_message_to_proto(m);
        assert_eq!(p.author_name, "Joe");
        assert_eq!(p.author_role, "user");
        assert_eq!(p.content, "Help!");
        assert_eq!(p.created_at, ts().to_rfc3339());
    }

    #[test]
    fn ticket_detail_to_proto_includes_ticket_and_messages() {
        let detail = TicketDetail {
            ticket: sample_ticket(),
            messages: vec![
                TicketMessage {
                    id: Uuid::nil(), ticket_id: Uuid::nil(),
                    author_name: "Joe".into(), author_role: "user".into(),
                    content: "msg1".into(), created_at: ts(),
                },
                TicketMessage {
                    id: Uuid::nil(), ticket_id: Uuid::nil(),
                    author_name: "Mod".into(), author_role: "moderator".into(),
                    content: "msg2".into(), created_at: ts(),
                },
            ],
        };
        let p = ticket_detail_to_proto(detail);
        assert!(p.ticket.is_some());
        assert_eq!(p.messages.len(), 2);
        assert_eq!(p.messages[1].author_role, "moderator");
    }
