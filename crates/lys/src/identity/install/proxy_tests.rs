use super::{LISTENING, unit};
use crate::identity::install::layout::Layout;
use crate::identity::install::ports::Ports;

#[test]
fn the_proxy_unit_runs_lys_proxy_serve_on_the_recorded_listener_inside_the_root() {
    let layout = Layout::at("/root-fixture".into());
    let ports = Ports::default();
    let unit = unit(&layout, ports);
    assert_eq!(unit.binary, "lys");
    assert_eq!(
        unit.args,
        [
            "proxy",
            "serve",
            "--listen",
            "127.0.0.1:8484",
            "--home",
            "/root-fixture/data/proxy/home",
            "--state",
            "/root-fixture/data/proxy/state",
        ]
    );
    assert_eq!(unit.pid, layout.run_dir().join("proxy.pid"));
    assert_eq!(unit.log, layout.logs_dir().join("proxy.log"));
    assert_eq!(unit.ready.says, LISTENING);
    assert!(unit.ready.answers.is_none());
}

#[test]
fn the_proxy_s_start_line_carries_the_words_its_unit_waits_for() {
    let line = serde_json::json!({
        "proxy": "listening",
        "listen": "127.0.0.1:8484",
        "anthropic": "https://api.anthropic.com",
        "anthropic_from": "default",
        "openai": "https://api.openai.com",
    })
    .to_string();
    assert!(line.contains(LISTENING), "{line}");
}
