extern crate corosync_config_parser;

#[test]
fn test_parse_example_file() {
    let corosync_example = "
        logging {
            fileline: off
            to_stderr: no
            to_logfile: no
            logfile: /var/log/cluster/corosync.log
            to_syslog: yes
            debug: off
            timestamp: on
            logger_subsys {
                    subsys: QUORUM
                    debug: off
            }
        }
    "
    .to_string();

    let cfg = corosync_config_parser::parse(corosync_example).unwrap();
    assert_eq!(cfg.name(), "");
    assert_eq!(cfg.len(), 0);
    let subsys = cfg
        .matching("logging")
        .nth(0)
        .unwrap()
        .matching("logger_subsys")
        .nth(0)
        .unwrap()
        .matching("subsys")
        .nth(0)
        .unwrap()
        .get(0);
    assert_eq!(subsys, "QUORUM");
}

#[test]
fn test_datapath_get() {
    let corosync_example = "
        logging {
            fileline: off
            to_stderr: no
            to_logfile: no
            logfile: /var/log/cluster/corosync.log
            to_syslog: yes
            debug: off
            timestamp: on
            logger_subsys {
                    subsys: QUORUM
                    debug: off
            }
        }
    "
    .to_string();

    let cfg = corosync_config_parser::parse(corosync_example).unwrap();
    let subsys = cfg.path(vec!["logging", "logger_subsys", "subsys"]);
    assert_eq!(subsys, Some("QUORUM"));
}
