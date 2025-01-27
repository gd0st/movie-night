use clap::{self, Parser};
use std::{
    net::{Ipv4Addr, SocketAddrV4},
    path::PathBuf,
    sync::Mutex,
};

use actix_cors;
use actix_web::{self, web::Data, App, HttpServer};
use movie_night_api::{app, routes};

#[derive(clap::Parser, Debug)]
struct Arg {
    #[arg(long, short)]
    socket: Option<SocketAddrV4>,
    #[arg(short, long)]
    config: PathBuf,
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let args = Arg::parse();

    let socket = args
        .socket
        .unwrap_or(SocketAddrV4::new(Ipv4Addr::new(127, 0, 0, 1), 5789));

    let Ok(config) = app::Config::try_from(args.config) else {
        eprintln!("error while parsing config");
        return Ok(());
    };

    let polls = Data::new(Mutex::new(config.make_polls()));
    let server = HttpServer::new(move || {
        App::new()
            .service(routes::get_poll)
            .service(routes::health_check)
            .service(routes::submit_new_form)
            // I need to tweak cors later probably.
            .wrap(actix_cors::Cors::permissive())
            .app_data(polls.clone())
    })
    .bind(socket)?;

    server
        .addrs()
        .iter()
        .for_each(|addr| println!("server running at {}:{} ", &addr.ip(), &addr.port()));

    server.run().await
}
