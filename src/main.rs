use std::convert::Infallible;
use std::ffi::OsString;
use std::fmt::write;
use std::net::SocketAddr;

use http::{Method, StatusCode, Version};
use hyper::body::{Bytes, HttpBody};
use hyper::server::conn::AddrStream;
use hyper::service::{make_service_fn, service_fn};
use hyper::{Body, Request, Response, Server};
use std::{io::Error, path::PathBuf};
use tokio::fs;
use tokio::io::{AsyncWrite, AsyncWriteExt};

fn main() {
    match server() {
        Ok(_) => {}
        Err(e) => println!("{e}"),
    };
}

#[tokio::main]
pub async fn server() -> Result<(), Infallible> {
    let make_svc = make_service_fn(move |conn: &AddrStream| {
        let addr = conn.remote_addr();

        let service = service_fn(move |req| serve_request(req, addr));
        async move { Ok::<_, Infallible>(service) }
    });

    let addr = ([127, 0, 0, 1], 7878).into();
    let server = Server::bind(&addr).serve(make_svc);
    let graceful = server.with_graceful_shutdown(shutdown_signal());

    if let Err(e) = graceful.await {
        eprintln!("server error: {e}");
    }

    Ok(())
}

async fn shutdown_signal() {
    tokio::signal::ctrl_c()
        .await
        .expect("failed to install CTRL+C signal handler")
}

async fn serve_request(req: Request<Body>, addr: SocketAddr) -> Result<Response<Body>, Infallible> {
    match (req.method(), req.uri().path()) {
        (&Method::GET, "/") => {
            let body = fs::read_to_string("main.html").await.unwrap();
            Ok(Response::builder()
                .header("Content-Type", "text/html")
                .body(Body::from(body))
                .unwrap())
        }
        (&Method::GET, "/main.css") => {
            let css_content = fs::read_to_string("main.css").await.unwrap_or_default();

            Ok(Response::builder()
                .header("Content-Type", "text/css")
                .body(Body::from(css_content))
                .unwrap())
        }
        (&Method::GET, "/self-hosting-a-website") => {
            let body = fs::read_to_string("article_11_23.html")
                .await
                .unwrap_or_default();

            Ok(Response::builder()
                .header("Content-Type", "text/html")
                .body(Body::from(body))
                .unwrap())
        }
        (&Method::GET, "/tip-your-server") => {
            let body = fs::read_to_string("article_05_05.html")
                .await
                .unwrap_or_default();

            Ok(Response::builder()
                .header("Content-Type", "text/html")
                .body(Body::from(body))
                .unwrap())
        }
        (&Method::GET, "/maths") => {
            let body = fs::read_to_string("maths.html").await.unwrap_or_default();

            Ok(Response::builder()
                .header("Content-Type", "text/html")
                .body(Body::from(body))
                .unwrap())
        }
        (&Method::GET, "/set") => {
            let toprint = req.uri().query().unwrap();
            // score=12?name=giammo
            let num: u32 = toprint[6..toprint.find('?').unwrap()].parse().unwrap();
            let name = &toprint[toprint.find('?').unwrap() + 6..];
            let csv_file = fs::read_to_string("leaderboard.csv").await;
            let mut lines: Vec<String> =
                csv_file.unwrap().lines().map(|x| String::from(x)).collect();
            let values: Vec<u32> = lines
                .iter()
                .flat_map(|x| x.split(';'))
                .filter_map(|x| x.parse::<u32>().ok())
                .collect();
            for n in 0..lines.len() - 1 {
                if *values.get(n).unwrap() < num {
                    lines.insert(n, String::from(num.to_string() + ";" + name));
                    break;
                }
            }
            let x = write_csv(lines).await;
            if x.is_err() {
                return error(404, None).await;
            }
            Ok(Response::new(Body::from("got it bbgirl")))
        }
        (&Method::GET, "/scoreboard") => {
            let body = fs::read_to_string("leaderboard.csv")
                .await
                .unwrap_or_default();

            Ok(Response::builder()
                .header("Content-Type", "text/html")
                .body(Body::from(body))
                .unwrap())
        }
        (&Method::GET, _) => other_content(&req).await,
        _ => error(404, None).await,
    }
}

pub async fn write_csv(to_write: Vec<String>) -> Result<(), String> {
    let mut file = fs::File::create("leaderboard.csv").await.unwrap();
    for n in 0..10 {
        file.write_all(to_write.get(n).unwrap().as_bytes()).await;
        file.write_all(b"\n").await;
    }
    file.flush().await;

    Ok(())
}

pub async fn error(code: u16, error: Option<Error>) -> Result<Response<Body>, Infallible> {
    let (response, status): (String, StatusCode) = match code {
        500 => (
            "server issues".to_string(),
            StatusCode::INTERNAL_SERVER_ERROR,
        ),
        404 => {
            let response = match fs::read_to_string("404.html").await {
                Ok(file) => file,
                Err(_) => {
                    return Ok(Response::new(Body::from(
                        "Well, that wasn't supposed to happen",
                    )))
                }
            };
            (response, StatusCode::NOT_FOUND)
        }
        _ => panic!("status code not accounted for"),
    };
    let mut response: Response<Body> = Response::new(Body::from(response));
    *response.status_mut() = status;
    Ok(response)
}

pub async fn media_response(x: PathBuf) -> Result<Vec<u8>, String> {
    let mut buffer = Vec::new();
    if tokio::fs::read(x.clone()).await.is_ok() {
        buffer = tokio::fs::read(x).await.unwrap();
        Ok(buffer)
    } else {
        Err(String::from("something went wrong while reading the file"))
    }
}

pub async fn other_content(request: &Request<Body>) -> Result<Response<Body>, Infallible> {
    let (dir_path, content_type) = if request.uri().path().contains("img") {
        let dir_path = "img";
        let content_type = "image/png";
        (dir_path, content_type)
    } else if request.uri().path().contains("pdf") {
        let dir_path = "pdf";
        let content_type = "application/pdf";
        (dir_path, content_type)
    } else {
        return Ok(error(404, None).await.unwrap());
    };

    let mut entries = fs::read_dir(dir_path).await.unwrap();
    let mut names = Vec::new();
    while let Some(entry) = entries.next_entry().await.unwrap() {
        names.push(entry.file_name())
    }
    let tosearch = OsString::from(&request.uri().path()[5..]).clone();

    if names.contains(&tosearch) {
        let temp_path = format!("{}{}{}", dir_path, "/", tosearch.to_str().unwrap());
        let path = PathBuf::from(temp_path);
        let tosearch = media_response(path).await;
        match tosearch {
            Ok(img) => Ok(Response::builder()
                .header("Content-Type", content_type)
                .body(Body::from(img))
                .unwrap()),
            _ => error(500, None).await,
        }
    // add pdf support via "else if" req uri contains pdf
    } else {
        error(404, None).await
    }
}
