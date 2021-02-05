mod net;
mod packets;
mod plugins;
mod threads;

use clap;
use plugins::LuaPluginInterface;

fn main() {
    let matches = clap::App::new("OpenNetBattle Server")
        .arg(
            clap::Arg::with_name("port")
                .short("p")
                .long("port")
                .value_name("PORT")
                .default_value("8765")
                .required(true)
                .takes_value(true)
                .validator(|value| {
                    let error_message = "PORT must be > 0 and < 65535";
                    let port = value
                        .parse::<u16>()
                        .map_err(|_| String::from(error_message))?;

                    if port != 0 {
                        Ok(())
                    } else {
                        Err(String::from(error_message))
                    }
                }),
        )
        .arg(
            clap::Arg::with_name("log_connections")
                .long("log-connections")
                .help("Logs connects and disconnects"),
        )
        .arg(
            clap::Arg::with_name("log_packets")
                .long("log-packets")
                .help("Logs received packets (useful for debugging)"),
        )
        .get_matches();

    // validators makes this safe to unwrap
    let config = net::ServerConfig {
        port: matches.value_of("port").unwrap().parse().unwrap(),
        log_connections: matches.is_present("log_connections"),
        log_packets: matches.is_present("log_packets"),
    };

    let mut server = net::Server::new(config);

    server.add_plugin_interface(Box::new(LuaPluginInterface::new()));

    if let Err(err) = server.start() {
        panic!("{}", err);
    }
}
