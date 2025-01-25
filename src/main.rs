use clap::{self, Parser};
use std::{
    fs::OpenOptions,
    net::{IpAddr, Ipv4Addr, SocketAddrV4},
    os::unix::net::SocketAddr,
    path::PathBuf,
    rc::Rc,
    sync::Mutex,
};

use actix_cors;
use actix_web::{
    self,
    web::{route, Data},
    App, HttpServer,
};
use movie_night_api::{
    app,
    polling::{self, Poll},
    routes,
};

#[derive(clap::Parser, Debug)]
struct Arg {
    #[arg(short, long)]
    port: Option<u16>,
    #[arg(short, long)]
    config: PathBuf,
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let args = Arg::parse();
    let ip = "127.0.0.1";
    let port = args.port.unwrap_or(5789);

    // TODO needs Args for ip, port & config parameter passing.

    let Ok(config) = app::Config::try_from(args.config) else {
        eprintln!("error while parsing config");
        return Ok(());
    };

    let polls = Data::new(Mutex::new(config.make_polls()));
    let server = HttpServer::new(move || {
        App::new()
            .service(routes::get_poll)
            .service(routes::health_check)
            .service(routes::form_submit)
            .service(routes::submit_new_form)
            // I need to tweak cors later probably.
            .wrap(actix_cors::Cors::permissive())
            .app_data(polls.clone())
    })
    .bind((ip, port))?;

    server
        .addrs()
        .iter()
        .for_each(|addr| println!("server running at {}:{} ", &addr.ip(), &addr.port()));

    server.run().await
}
