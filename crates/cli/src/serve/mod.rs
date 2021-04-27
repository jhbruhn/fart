mod events;

use crate::{
    command_ext::CommandExt, output::Output, sub_command::SubCommand, watcher::Watcher, Result,
};
use failure::ResultExt;
use futures::{channel::mpsc, FutureExt, TryFutureExt};
use std::collections::HashMap;
use std::env;
use std::ffi::OsStr;
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::sync::{Arc, Mutex};
use std::thread;
use structopt::StructOpt;

/// Serve a fart project over a local server, watch it for changes, and re-build
/// and re-un it as necessary.
#[derive(Clone, Debug, StructOpt)]
pub struct Serve {
    /// The project to serve.
    #[structopt(parse(from_os_str), default_value = ".")]
    project: PathBuf,

    /// The port to serve locally on.
    #[structopt(short = "p", long = "port", default_value = "9090")]
    port: u16,

    /// Extra arguments passed along to each invocation of `cargo run`.
    #[structopt(long = "")]
    extra: Vec<String>,
}

impl Serve {
    fn app_data(&mut self) -> AppData {
        AppData {
            project: self.project.clone(),
            subscribers: Arc::new(Mutex::new(HashMap::new())),
            consts: Arc::new(Mutex::new(HashMap::new())),
        }
    }
}

impl SubCommand for Serve {
    fn set_extra(&mut self, extra: &[String]) {
        assert!(self.extra.is_empty());
        self.extra = extra.iter().cloned().collect();
    }

    fn run(mut self) -> Result<()> {
        let app_data = self.app_data();

        let subscribers = app_data.subscribers.clone();
        let project = self.project.clone();
        let extra = self.extra.clone();

        let mut output_buffer = String::new();
        let mut parsing_params = false;
        thread::spawn(move || {
            Watcher::new(project)
                .extra(extra)
                .on_output({
                    let subscribers = subscribers.clone();
                    move |output| {
                        output_buffer.push_str(output);
                        if output.contains("fart: PARAMS START") {
                            parsing_params = true;
                        }
                        if output.contains("fart: PARAMS END") {
                            parsing_params = false;
                        }
                        if !parsing_params {
                            let send_output = || -> Result<()> {
                                let event = events::Event::new("output".into(), &output_buffer)
                                    .context("failed to serialize output event")?;
                                futures::executor::block_on(events::broadcast(
                                    &subscribers,
                                    event,
                                ))?;
                                Ok(())
                            };
                            if let Err(e) = send_output() {
                                eprintln!("warning: {}", e);
                            }
                            output_buffer.clear();
                        }
                    }
                })
                .on_start({
                    let subscribers = subscribers.clone();
                    move || {
                        let send_rerun = || -> Result<()> {
                            let event = events::Event::new("start".into(), &())
                                .context("failed to serialize rerun event")?;
                            futures::executor::block_on(events::broadcast(&subscribers, event))?;
                            Ok(())
                        };
                        if let Err(e) = send_rerun() {
                            eprintln!("warning: {}", e);
                        }
                    }
                })
                .on_finish({
                    let subscribers = subscribers.clone();
                    move || {
                        let send_rerun = || -> Result<()> {
                            let event = events::Event::new("finish".into(), &())
                                .context("failed to serialize rerun event")?;
                            futures::executor::block_on(events::broadcast(&subscribers, event))?;
                            Ok(())
                        };
                        if let Err(e) = send_rerun() {
                            eprintln!("warning: {}", e);
                        }
                    }
                })
                .watch()
                .unwrap();
        });

        let mut app = tide::Server::with_state(app_data);
        app.at("/").get(serve_from_memory(
            "text/html",
            include_str!("static/index.html"),
        ));
        app.at("/styles.css").get(serve_from_memory(
            "text/css",
            include_str!("static/styles.css"),
        ));
        app.at("/script.js").get(serve_from_memory(
            "text/javascript",
            include_str!("static/script.js"),
        ));
        app.at("/events").get(events);
        app.at("/rerun").post(rerun);
        app.at("/like").post(like);
        app.at("/images/:image").get(image);
        async_std::task::block_on(
            app.listen(format!("127.0.0.1:{}", self.port))
                .map_err(|_| ())
                .boxed(),
        )
        .map_err(|()| failure::format_err!("failed to listen on port {}", self.port))?;

        Ok(())
    }
}

#[derive(Clone)]
struct AppData {
    project: PathBuf,
    subscribers: Arc<Mutex<HashMap<usize, mpsc::Sender<events::Event>>>>,
    consts: Arc<Mutex<HashMap<String, String>>>,
}

fn serve_from_memory(
    content_type: &'static str,
    body: &'static str,
) -> impl tide::Endpoint<AppData> {
    return ServeFromMemory { content_type, body };

    struct ServeFromMemory {
        content_type: &'static str,
        body: &'static str,
    }

    #[async_trait::async_trait]
    impl<T> tide::Endpoint<T> for ServeFromMemory
    where
        T: Clone + Send + Sync + 'static,
    {
        async fn call(&self, _cx: tide::Request<T>) -> tide::Result<tide::Response> {
            let mut res = tide::Response::new(200);
            res.insert_header("Content-Type", self.content_type);
            res.set_body(tide::Body::from_string(self.body.to_string()));
            Ok(res)
        }
    }
}

async fn events(cx: tide::Request<AppData>) -> tide::Result<tide::Response> {
    let events = events::EventStream::new(cx.state().subscribers.clone());
    let mut res = tide::Response::new(200);
    res.set_body(tide::Body::from_reader(events, None));
    res.insert_header("Content-Type", "text/event-stream");
    res.insert_header("X-Accel-Buffering", "no");
    res.insert_header("Cache-Control", "no-cache");
    Ok(res)
}

async fn rerun(mut cx: tide::Request<AppData>) -> tide::Result<tide::Response> {
    let mut response = tide::Response::new(200);

    let vars: HashMap<String, String> = match cx.body_json().await {
        Ok(vars) => vars,
        Err(e) => {
            let mut res = tide::Response::new(tide::http::StatusCode::BadRequest);
            res.set_body(tide::Body::from_string(e.to_string()));
            return Ok(res);
        }
    };

    let touched = {
        let mut consts = cx.state().consts.lock().unwrap();

        for (k, v) in vars {
            let k = format!("FART_USER_CONST_{}", k);
            env::set_var(&k, &v);
            consts.insert(k, v);
        }

        let mut vars = "# fart user consts\n\
                        #\n\
                        # To re-establish this user const environment, run:\n\
                        #\n\
                        #    $ source user_consts.sh\n\n\
                        "
        .to_string();
        for (k, v) in consts.iter() {
            vars.push_str(&format!("export {}={}\n", k, v));
        }

        let vars_path = cx.state().project.join("user_consts.sh");
        let wrote_consts =
            fs::write(vars_path, vars.as_bytes()).map_err(|e| failure::Error::from(e));

        wrote_consts.and_then(|_| {
            // Touch the `src` directory to get the watcher to rebuild. Kinda hacky but
            // it works!
            let src = cx.state().project.join("src");
            Command::new("touch")
                .arg(src)
                .run_result(&mut Output::Inherit)
        })
    };

    match touched {
        Ok(_) => response.set_body(tide::Body::from_string("".to_string())),
        Err(e) => {
            response.set_body(tide::Body::from_string(e.to_string()));
            response.set_status(tide::http::StatusCode::InternalServerError);
        }
    };
    Ok(response)
}

async fn image(cx: tide::Request<AppData>) -> tide::Result<tide::Response> {
    let image = PathBuf::from(cx.param("image").unwrap());
    if image.extension() != Some(OsStr::new("svg")) {
        return Ok(tide::Response::new(404));
    }
    let path = cx.state().project.join("images").join(image);
    serve_static_file(path).await
}

async fn serve_static_file(path: PathBuf) -> tide::Result<tide::Response> {
    let mut res = tide::Response::new(200);
    res.set_body(tide::Body::from_file(path).await?);
    Ok(res)
}

async fn like(cx: tide::Request<AppData>) -> tide::Result<tide::Response> {
    let now = chrono::Utc::now();
    let now = now.format("%Y-%m-%d-%H-%M-%S-%f").to_string();

    let like_name = format!("liked_{}.svg", now);
    let liked_path = cx.state().project.join("liked/");
    std::fs::create_dir_all(&liked_path).unwrap();
    let liked_path = liked_path.join(like_name);

    let latest_path = cx.state().project.join("images").join("latest.svg");

    println!("Latest: {:?} Liked: {:?}", &latest_path, &liked_path);

    std::fs::copy(&latest_path, &liked_path).unwrap();

    crate::git::add_all(&cx.state().project, &mut crate::output::Output::Inherit).unwrap();
    crate::git::commit(
        &cx.state().project,
        &format!("Liked {}", now),
        &mut crate::output::Output::Inherit,
    )
    .unwrap();

    Ok(tide::Response::new(200))
}
