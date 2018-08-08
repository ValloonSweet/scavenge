extern crate log;
extern crate log4rs;
use config::Cfg;

use log::LevelFilter;
use log4rs::append::console::ConsoleAppender;
use log4rs::append::rolling_file::policy::compound::roll::fixed_window::FixedWindowRoller;
use log4rs::append::rolling_file::policy::compound::trigger::size::SizeTrigger;
use log4rs::append::rolling_file::policy::compound::CompoundPolicy;
use log4rs::append::rolling_file::RollingFileAppender;
use log4rs::config::{Appender, Config, Root};
use log4rs::encode::pattern::PatternEncoder;
use log4rs::filter::threshold::ThresholdFilter;

fn to_log_level(s: &String, default: log::LevelFilter) -> log::LevelFilter {
    match s.to_lowercase().as_str() {
        "trace" => log::LevelFilter::Trace,
        "debug" => log::LevelFilter::Debug,
        "info" => log::LevelFilter::Info,
        "warn" => log::LevelFilter::Warn,
        "error" => log::LevelFilter::Error,
        "off" => log::LevelFilter::Off,
        _ => default,
    }
}

pub fn init_logger(cfg: &Cfg) -> log4rs::Handle {
    let level_console = to_log_level(&cfg.console_log_level, log::LevelFilter::Info);
    let level_logfile = to_log_level(&cfg.logfile_log_level, log::LevelFilter::Warn);

    let stdout = ConsoleAppender::builder()
        .encoder(Box::new(PatternEncoder::new(
            "{({d(%H:%M:%S)} [{l}]):16.16} {m}{n}",
        ))).build();

    let roller = FixedWindowRoller::builder()
        .base(1)
        .build("log/scavenger.{}.log", cfg.logfile_max_count)
        .unwrap();
    let trigger = SizeTrigger::new(&cfg.logfile_max_size * 1024 * 1024);
    let policy = Box::new(CompoundPolicy::new(Box::new(trigger), Box::new(roller)));

    let config: Config;
    if level_logfile == log::LevelFilter::Off {
        config = Config::builder()
            .appender(
                Appender::builder()
                    .filter(Box::new(ThresholdFilter::new(level_console)))
                    .build("stdout", Box::new(stdout)),
            ).build(Root::builder().appender("stdout").build(LevelFilter::Info))
            .unwrap();
    } else {
        let logfile = RollingFileAppender::builder()
            .encoder(Box::new(PatternEncoder::new(
                "{({d(%Y-%m-%d %H:%M:%S)} [{l}]):26.26} {m}{n}",
            ))).build("log/scavenger.1.log", policy)
            .unwrap();
        config = Config::builder()
            .appender(
                Appender::builder()
                    .filter(Box::new(ThresholdFilter::new(level_console)))
                    .build("stdout", Box::new(stdout)),
            ).appender(
                Appender::builder()
                    .filter(Box::new(ThresholdFilter::new(level_logfile)))
                    .build("logfile", Box::new(logfile)),
            ).build(
                Root::builder()
                    .appender("stdout")
                    .appender("logfile")
                    .build(LevelFilter::Info),
            ).unwrap();
    }

    log4rs::init_config(config).unwrap()
}
