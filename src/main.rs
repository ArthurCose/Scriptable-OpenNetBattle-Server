mod net;
mod packets;
mod plugins;
mod threads;

use clap;
use net::Server;
use plugins::LuaPluginInterface;

fn main() {
    let matches = clap::App::new("OpenNetBattle Server")
        .about("https://github.com/TheMaverickProgrammer/OpenNetBattle")
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
            clap::Arg::with_name("log_packets")
                .short("l")
                .long("log-packets")
                .help("Outputs received packets (useful for debugging)"),
        )
        .get_matches();

    // validators makes this safe to unwrap
    let port: u16 = matches.value_of("port").unwrap().parse().unwrap();
    let log_packets = matches.is_present("log_packets");

    let mut server = Server::new(port, log_packets);

    server.add_plugin_interface(Box::new(LuaPluginInterface::new()));

    if let Err(err) = server.start() {
        panic!("{}", err);
    }
}
