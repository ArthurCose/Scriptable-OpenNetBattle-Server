mod helpers;
mod net;
mod packets;
mod plugins;
mod threads;

use clap;
use helpers::unwrap_and_parse_or_default;
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
            clap::Arg::with_name("max_payload_size")
                .long("max-payload-size")
                .help("Maximum data size a packet can carry, excluding UDP headers (reduce for lower packet drop rate)")
                .value_name("SIZE_IN_BYTES")
                .default_value("10240")
                .takes_value(true)
                .validator(|value| {
                    let error_message = "Invalid payload size";
                    let max_payload_size = value
                        .parse::<u16>()
                        .map_err(|_| String::from(error_message))?;

                    // max size defined by NetPlayConfig::MAX_BUFFER_LEN
                    if max_payload_size >= 100 && max_payload_size <= 10240 {
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
                .long("player-asset-limit")
                .help("Sets the file size limit for avatar files (in KiB)")
                .value_name("SIZE_IN_KiB")
                .default_value("5120") // 5 MiB - todo: reduce to 1 MiB?
                .takes_value(true)
                .validator(|value| match value.parse::<usize>() {
                    Ok(_) => Ok(()),
                    Err(_) => Err(String::from("Invalid file size")),
                }),
        )
        .get_matches();

    let config = net::ServerConfig {
        // validators makes this safe to unwrap
        port: matches.value_of("port").unwrap().parse().unwrap(),
        max_payload_size: unwrap_and_parse_or_default(matches.value_of("max_payload_size")),
        log_connections: matches.is_present("log_connections"),
        log_packets: matches.is_present("log_packets"),
        player_asset_limit: unwrap_and_parse_or_default::<usize>(
            matches.value_of("player_asset_limit"),
        ) * 1024,
    };

    let mut server = net::Server::new(config);

    server.add_plugin_interface(Box::new(LuaPluginInterface::new()));

    if let Err(err) = server.start() {
        panic!("{}", err);
    }
}
