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
        .arg(
            clap::Arg::with_name("player_asset_limit")
                .default_value("5120") // 5 MiB - todo: reduce to 1 MiB?
                .long("player-asset-limit")
                .takes_value(true)
                .help("Sets the file size limit for avatar files (in KiB)")
                .validator(|value| match value.parse::<usize>() {
                    Ok(_) => Ok(()),
                    Err(_) => Err(String::from("Invalid file size")),
                }),
        )
        .get_matches();

    let config = net::ServerConfig {
        // validators makes this safe to unwrap
        port: matches.value_of("port").unwrap().parse().unwrap(),
        log_connections: matches.is_present("log_connections"),
        log_packets: matches.is_present("log_packets"),
        player_asset_limit: matches
            .value_of("player_asset_limit")
            .unwrap()
            .parse::<usize>()
            .unwrap()
            * 1024,
    };

    let mut server = net::Server::new(config);

    server.add_plugin_interface(Box::new(LuaPluginInterface::new()));

    if let Err(err) = server.start() {
        panic!("{}", err);
    }
}
