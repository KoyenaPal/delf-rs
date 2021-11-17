extern crate clap;
extern crate mysql;
#[macro_use]
extern crate rocket;
extern crate lettre;
extern crate lettre_email;
#[macro_use]
extern crate slog;
extern crate slog_term;
#[macro_use]
extern crate serde_derive;

mod admin;
mod apikey;
mod args;
mod backend;
mod config;
mod email;
mod login;
mod questions;

// use clap::Clap;

// use delf;
use backend::MySqlBackend;
//use rocket::fs::FileServer;
use rocket::http::CookieJar;
use rocket::response::Redirect;
use rocket::State;
use rocket_dyn_templates::Template;
use std::sync::{Arc, Mutex};

// #[derive(Clap)]
// struct Opts {
//     #[clap(subcommand)]
//     subcmd: SubCommand,
//     /// Sets a custom config file. Could have been an Option<T> with no default too
//     #[clap(short, long, default_value = "config.yaml")]
//     config: String,
//     /// Some input. Because this isn't an Option<T> it's required to be used
//     #[clap(short, long, default_value = "schema.yaml")]
//     schema: String,
// }

// #[derive(Clap)]
// enum SubCommand {
//     Validate,
//     Run,
// }


pub fn new_logger() -> slog::Logger {
    use slog::Drain;
    use slog::Logger;
    use slog_term::term_full;
    Logger::root(Mutex::new(term_full()).fuse(), o!())
}

#[get("/")]
fn index(cookies: &CookieJar<'_>, backend: &State<Arc<Mutex<MySqlBackend>>>) -> Redirect {
    if let Some(cookie) = cookies.get("apikey") {
        let apikey: String = cookie.value().parse().ok().unwrap();
        // TODO validate API key
        match apikey::check_api_key(&*backend, &apikey) {
            Ok(_user) => Redirect::to("/leclist"),
            Err(_) => Redirect::to("/login"),
        }
    } else {
        Redirect::to("/login")
    }
}

#[rocket::main]
async fn main() {
    let args = args::parse_args();
    let config = args.config;

    let backend = Arc::new(Mutex::new(
        MySqlBackend::new(&format!("{}", args.class), Some(new_logger()), config.prime).unwrap(),
    ));

    //let template_dir = config.template_dir.clone();
    //let resource_dir = config.resource_dir.clone();

    if let Err(e) = rocket::build()
        .attach(Template::fairing())
        .manage(backend)
        .manage(config)
        //.mount("/css", FileServer::from(format!("{}/css", resource_dir)))
        //.mount("/js", FileServer::from(format!("{}/js", resource_dir)))
        .mount("/", routes![index])
        .mount(
            "/questions",
            routes![questions::questions, questions::questions_submit],
        )
        .mount("/apikey/check", routes![apikey::check])
        .mount("/apikey/generate", routes![apikey::generate])
        .mount("/answers", routes![questions::answers])
        .mount("/leclist", routes![questions::leclist])
        .mount("/login", routes![login::login])
        .mount(
            "/admin/lec/add",
            routes![admin::lec_add, admin::lec_add_submit],
        )
        .mount("/admin/users", routes![admin::get_registered_users])
        .mount(
            "/admin/lec",
            routes![admin::lec, admin::addq, admin::editq, admin::editq_submit],
        )
        .launch()
        .await
    {
        println!("Whoops, didn't launch!");
        drop(e);
    };

    // let opts: Opts = Opts::parse();

    // match opts.subcmd {
    //     SubCommand::Validate => {
    //         println!("Validating schema...");
    //         validate(&opts.schema, &opts.config);
    //     }
    //     SubCommand::Run => {
    //         println!("Starting delf api...");
    //         run(&opts.schema, &opts.config);
    //     }
    // }
}

// fn validate(schema_path: &String, config_path: &String) {
//     let graph = delf::read_files(schema_path, config_path);
//     graph.validate();
// }

// fn run(schema_path: &String, config_path: &String) {
//     delf::check_short_ttl_loop(schema_path, config_path);
//     delf::init_api(schema_path, config_path).launch();
// }
