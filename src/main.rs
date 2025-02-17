use clap::{self, Parser};
use openssl::{
    self,
    pkey::{PKey, Private},
    ssl::{SslAcceptor, SslFiletype, SslMethod},
};
use std::{
    fs::OpenOptions,
    io::Read,
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

    #[arg(long)]
    private_pem: PathBuf,
    #[arg(long)]
    cert_pem: PathBuf,
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let args = Arg::parse();

    let mut ssl = SslAcceptor::mozilla_intermediate(SslMethod::tls())?;
    ssl.set_private_key_file(args.private_pem, SslFiletype::PEM)?;
    ssl.set_certificate_file(args.cert_pem, SslFiletype::PEM)?;

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
    // .bind(socket)?;
    .bind_openssl(socket, ssl)?;

    server.run().await
}
