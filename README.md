# corosync-config-parser
A Rust crate for hassle-free Corosync's configuration file parsing.

Inspired by [Kilobyte22/config-parser](https://github.com/Kilobyte22/config-parser).

## Usage
```rust
extern crate corosync_config_parser;

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
```
