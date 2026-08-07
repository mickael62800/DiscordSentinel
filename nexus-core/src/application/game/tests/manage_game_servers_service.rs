use super::render_template;
use std::collections::HashMap;

#[test]
fn test_render_template_placeholders() {
    let mut env = HashMap::new();
    env.insert("WORLD_NAME".to_string(), "MyWorld".to_string());
    env.insert("MAX_PLAYERS".to_string(), "10".to_string());

    let template = "Server name: {{ WORLD_NAME }}, max: {{MAX_PLAYERS}}, unset: {{ UNSET_VAR }}";
    let rendered = render_template(template, &env);

    assert_eq!(rendered, "Server name: MyWorld, max: 10, unset: ");
}

#[test]
fn test_render_template_unclosed() {
    let env = HashMap::new();
    let template = "Hello {{ UNCLOSED";
    let rendered = render_template(template, &env);

    assert_eq!(rendered, "Hello {{ UNCLOSED");
}
