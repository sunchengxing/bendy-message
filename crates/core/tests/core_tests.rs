#[cfg(test)]
mod tests {
    use bmsg_core::*;

    fn make_target(platform: &str, app: &str, user: &str, mt: MessageType) -> Target {
        Target { platform: platform.into(), app_package: app.into(), user_id: user.into(), msg_type: mt }
    }

    fn make_service(id: &str, app: &str, platforms: Vec<&str>) -> RegisteredService {
        RegisteredService {
            id: id.into(), name: format!("svc-{}", id), endpoint: format!("https://{}.example.com", id),
            app_package: app.into(), platforms: platforms.into_iter().map(String::from).collect(),
            secret_hash: "hash".into(), status: ServiceStatus::Online, last_heartbeat: 0, created_at: 0,
        }
    }

    // --- MessageType ---
    #[test]
    fn message_type_as_str() {
        assert_eq!(MessageType::Notification.as_str(), "notification");
        assert_eq!(MessageType::Message.as_str(), "message");
        assert_eq!(MessageType::Shell.as_str(), "shell");
    }

    #[test]
    fn message_type_display() {
        assert_eq!(format!("{}", MessageType::Notification), "notification");
        assert_eq!(format!("{}", MessageType::Shell), "shell");
    }

    #[test]
    fn message_type_serde() {
        let mt = MessageType::Message;
        let json = serde_json::to_string(&mt).unwrap();
        assert_eq!(json, "\"message\"");
        let de: MessageType = serde_json::from_str(&json).unwrap();
        assert_eq!(de, mt);
    }

    // --- Target ---
    #[test]
    fn target_exact_match() {
        let t1 = make_target("ios", "com.app", "u1", MessageType::Notification);
        let t2 = make_target("ios", "com.app", "u1", MessageType::Notification);
        assert!(t1.matches(&t2));
    }

    #[test]
    fn target_platform_mismatch() {
        let t1 = make_target("ios", "com.app", "u1", MessageType::Notification);
        let t2 = make_target("android", "com.app", "u1", MessageType::Notification);
        assert!(!t1.matches(&t2));
    }

    #[test]
    fn target_msg_type_mismatch() {
        let t1 = make_target("ios", "com.app", "u1", MessageType::Notification);
        let t2 = make_target("ios", "com.app", "u1", MessageType::Shell);
        assert!(!t1.matches(&t2));
    }

    #[test]
    fn target_wildcard_platform() {
        let t1 = make_target("*", "com.app", "u1", MessageType::Notification);
        let t2 = make_target("android", "com.app", "u1", MessageType::Notification);
        assert!(t1.matches(&t2));
    }

    #[test]
    fn target_wildcard_app() {
        let t1 = make_target("ios", "*", "u1", MessageType::Notification);
        let t2 = make_target("ios", "com.other", "u1", MessageType::Notification);
        assert!(t1.matches(&t2));
    }

    #[test]
    fn target_wildcard_user() {
        let t1 = make_target("ios", "com.app", "*", MessageType::Message);
        let t2 = make_target("ios", "com.app", "u999", MessageType::Message);
        assert!(t1.matches(&t2));
    }

    #[test]
    fn target_all_wildcards() {
        let t1 = make_target("*", "*", "*", MessageType::Shell);
        let t2 = make_target("web", "com.x", "u42", MessageType::Shell);
        assert!(t1.matches(&t2));
    }

    // --- match_services ---
    #[test]
    fn match_services_exact() {
        let target = make_target("ios", "com.app", "u1", MessageType::Notification);
        let svcs = vec![
            make_service("s1", "com.app", vec!["ios"]),
            make_service("s2", "com.other", vec!["ios"]),
            make_service("s3", "com.app", vec!["android"]),
        ];
        let matched = match_services(&target, &svcs);
        assert_eq!(matched.len(), 1);
        assert_eq!(matched[0].id, "s1");
    }

    #[test]
    fn match_services_wildcard_platform() {
        let target = make_target("ios", "com.app", "u1", MessageType::Notification);
        let svcs = vec![
            make_service("s1", "com.app", vec!["*"]),
            make_service("s2", "com.app", vec!["android"]),
        ];
        let matched = match_services(&target, &svcs);
        assert_eq!(matched.len(), 1);
        assert_eq!(matched[0].id, "s1");
    }

    #[test]
    fn match_services_wildcard_app() {
        let target = make_target("ios", "com.app", "u1", MessageType::Notification);
        let svcs = vec![
            make_service("s1", "*", vec!["ios"]),
        ];
        let matched = match_services(&target, &svcs);
        assert_eq!(matched.len(), 1);
    }

    #[test]
    fn match_services_no_match() {
        let target = make_target("web", "com.app", "u1", MessageType::Notification);
        let svcs = vec![
            make_service("s1", "com.app", vec!["ios"]),
            make_service("s2", "com.other", vec!["web"]),
        ];
        let matched = match_services(&target, &svcs);
        assert!(matched.is_empty());
    }

    #[test]
    fn match_services_multiple_matches() {
        let target = make_target("ios", "com.app", "u1", MessageType::Notification);
        let svcs = vec![
            make_service("s1", "com.app", vec!["ios"]),
            make_service("s2", "com.app", vec!["*", "ios"]),
            make_service("s3", "com.app", vec!["android"]),
        ];
        let matched = match_services(&target, &svcs);
        assert_eq!(matched.len(), 2);
    }

    // --- build_delivery_payload ---
    #[test]
    fn delivery_payload_structure() {
        let msg = Message::new(
            make_target("ios", "com.app", "u1", MessageType::Notification),
            serde_json::json!({"text": "hello"}),
            true, None,
        );
        let payload = build_delivery_payload(&msg);
        assert_eq!(payload["target"]["platform"], "ios");
        assert_eq!(payload["target"]["app_package"], "com.app");
        assert_eq!(payload["target"]["user_id"], "u1");
        assert_eq!(payload["target"]["msg_type"], "notification");
        assert_eq!(payload["msg_type"], "notification");
        assert_eq!(payload["content"]["text"], "hello");
        assert_eq!(payload["persist"], true);
    }

    // --- Message ---
    #[test]
    fn message_new_generates_uuid() {
        let msg = Message::new(
            make_target("web", "com.app", "u1", MessageType::Message),
            serde_json::json!({}), false, None,
        );
        assert!(!msg.id.is_empty());
        assert_eq!(msg.id.len(), 36); // UUID format
        assert_eq!(msg.msg_type, MessageType::Message);
        assert!(!msg.persist);
        assert!(msg.ttl.is_none());
        assert!(msg.expires_at.is_none());
    }

    #[test]
    fn message_new_with_ttl() {
        let msg = Message::new(
            make_target("web", "com.app", "u1", MessageType::Shell),
            serde_json::json!({}), true, Some(3600),
        );
        assert_eq!(msg.ttl, Some(3600));
        assert!(msg.expires_at.is_some());
        assert_eq!(msg.expires_at.unwrap(), msg.created_at + 3600);
    }

    #[test]
    fn message_serde_roundtrip() {
        let msg = Message::new(
            make_target("ios", "com.app", "u1", MessageType::Notification),
            serde_json::json!({"body": "test"}), true, Some(60),
        );
        let json = serde_json::to_string(&msg).unwrap();
        let de: Message = serde_json::from_str(&json).unwrap();
        assert_eq!(de.id, msg.id);
        assert_eq!(de.target.platform, "ios");
        assert_eq!(de.persist, true);
    }

    // --- SendMessageRequest ---
    #[test]
    fn send_request_into_message() {
        let req = SendMessageRequest {
            target: make_target("android", "com.app", "u1", MessageType::Message),
            content: serde_json::json!({"text": "hi"}),
            persist: Some(false),
            ttl: Some(120),
        };
        let msg = req.into_message();
        assert_eq!(msg.target.platform, "android");
        assert!(!msg.persist);
        assert_eq!(msg.ttl, Some(120));
    }

    #[test]
    fn send_request_default_persist() {
        let req = SendMessageRequest {
            target: make_target("android", "com.app", "u1", MessageType::Message),
            content: serde_json::json!({}),
            persist: None,
            ttl: None,
        };
        let msg = req.into_message();
        assert!(msg.persist); // defaults to true
    }

    // --- ApiResponse ---
    #[test]
    fn api_response_success() {
        let resp: ApiResponse<i32> = ApiResponse::success(42);
        assert_eq!(resp.code, 0);
        assert_eq!(resp.message, "ok");
        assert_eq!(resp.data, Some(42));
    }

    #[test]
    fn api_response_error() {
        let resp = ApiResponse::<()>::error(1004, "not found".into());
        assert_eq!(resp.code, 1004);
        assert_eq!(resp.message, "not found");
        assert!(resp.data.is_none());
    }

    // --- BmsgError ---
    #[test]
    fn error_codes() {
        assert_eq!(BmsgError::NotFound.code(), 1004);
        assert_eq!(BmsgError::ServiceNotFound.code(), 1004);
        assert_eq!(BmsgError::Unauthorized.code(), 1002);
        assert_eq!(BmsgError::InvalidRequest("x".into()).code(), 1003);
        assert_eq!(BmsgError::StorageError("x".into()).code(), 1000);
        assert_eq!(BmsgError::ElectionError("x".into()).code(), 1000);
        assert_eq!(BmsgError::RegistryError("x".into()).code(), 1000);
        assert_eq!(BmsgError::Internal("x".into()).code(), 1000);
    }

    #[test]
    fn error_display() {
        assert_eq!(BmsgError::NotFound.to_string(), "message not found");
        assert_eq!(BmsgError::Unauthorized.to_string(), "unauthorized: invalid secret");
        let e = BmsgError::StorageError("disk full".into());
        assert!(e.to_string().contains("disk full"));
    }

    // --- ServiceStatus ---
    #[test]
    fn service_status_display() {
        assert_eq!(ServiceStatus::Online.to_string(), "online");
        assert_eq!(ServiceStatus::Offline.to_string(), "offline");
    }

    // --- NodeRole ---
    #[test]
    fn node_role_serde() {
        let json = serde_json::to_string(&NodeRole::Master).unwrap();
        assert_eq!(json, "\"master\"");
        let de: NodeRole = serde_json::from_str("\"slave\"").unwrap();
        assert_eq!(de, NodeRole::Slave);
    }

    // --- BatchSendMessageRequest ---
    #[test]
    fn batch_request() {
        let req = BatchSendMessageRequest {
            messages: vec![
                SendMessageRequest {
                    target: make_target("ios", "com.app", "u1", MessageType::Notification),
                    content: serde_json::json!({"a": 1}),
                    persist: Some(true), ttl: None,
                },
                SendMessageRequest {
                    target: make_target("android", "com.app", "u2", MessageType::Message),
                    content: serde_json::json!({"b": 2}),
                    persist: Some(false), ttl: Some(60),
                },
            ],
        };
        assert_eq!(req.messages.len(), 2);
        let msgs: Vec<Message> = req.messages.into_iter().map(|r| r.into_message()).collect();
        assert_eq!(msgs[0].persist, true);
        assert_eq!(msgs[1].persist, false);
        assert_eq!(msgs[1].ttl, Some(60));
    }
}
