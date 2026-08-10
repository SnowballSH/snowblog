use clap::Parser;
use snowblog_server::Config;

#[derive(Parser)]
struct Command {
    #[command(flatten)]
    config: Config,
}

// Break caught: making metrics configuration required, ignoring its CLI value, or accepting malformed addresses.
#[test]
fn metrics_listener_is_opt_in_and_validated() {
    let defaults = Command::try_parse_from(["snowblog-server", "--database", "blog.db"])
        .expect("the metrics listener is optional");
    assert_eq!(defaults.config.metrics_listen, None);

    let configured = Command::try_parse_from([
        "snowblog-server",
        "--database",
        "blog.db",
        "--metrics-listen",
        "127.0.0.1:9101",
    ])
    .expect("a valid metrics listener parses");
    assert_eq!(
        configured.config.metrics_listen,
        Some("127.0.0.1:9101".parse().unwrap())
    );

    assert!(
        Command::try_parse_from([
            "snowblog-server",
            "--database",
            "blog.db",
            "--metrics-listen",
            "not-a-socket-address",
        ])
        .is_err()
    );
}
