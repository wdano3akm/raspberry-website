use std::convert::Infallible;
use std::env::args;
use std::net::SocketAddr;


use std::io::{ Error, self};
use std::path::PathBuf;
use std::process::exit;
use std::sync::Arc;
use http::{Method, StatusCode};
use hyper::server::conn::AddrStream;
use hyper::{Body, Request, Response, Server};
use hyper::service::{make_service_fn, service_fn};
use tokio::sync::oneshot::Sender;
use tokio::{fs, fs::read_dir, sync::Mutex};

#[tokio::main]
async fn main() {

    NonBlockingStd::new().await;

    match server().await {
        Ok(_) => {},
        Err(e) => println!("{e}"),
    };
}

pub async fn server() -> Result<(), Infallible> {
    
    let make_svc = make_service_fn(move |conn: &AddrStream| {
        let addr = conn.remote_addr();

        let service = service_fn(move |req| { 
            serve_request( req ,addr)
        });
        async move {Ok::<_, Infallible>(service) }
    });

    let addr = ([127, 0, 0, 1], 7878).into();
    let server = Server::bind(&addr).serve(make_svc);

    // create oneshot mpsc to sent signal to gracefully shutdown server from CLI
    let (tx, rx) = tokio::sync::oneshot::channel::<()>();
    let graceful = server.with_graceful_shutdown(async {
        rx.await.ok();}
    );

    tokio::spawn(async move{
        // share transmitter with function
        NonBlockingStd::stdin_loop(tx).await;
        
    });

    if let Err(e) = graceful.await {
        eprintln!("server error: {e}");
    }

    Ok(())
}

async fn serve_request(req: Request<Body>, addr: SocketAddr) -> Result<Response<Body>, Infallible> {
    match (req.method(), req.uri().path()) {
        (&Method::GET, "/") => {
            let body = fs::read_to_string("main.html").await.unwrap();
            let body = body; 
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
            let body = fs::read_to_string("article_11_23.html").await.unwrap_or_default();

            Ok(Response::builder()
                .header("Content-Type", "text/html")
                .body(Body::from(body))
                .unwrap())
        }
        _ => error(404,None).await
 }
}

// make the program able to recieve input while working (thread)
// add functionality to scan current folder

pub async fn error(code: u16, error: Option<Error>) -> Result<Response<Body>, Infallible> {
    let (response, status): (String, StatusCode) =  match code {
        500 => ("server issues".to_string(), StatusCode::INTERNAL_SERVER_ERROR),
        404 => {
            let response = match fs::read_to_string("404.html").await {
                Ok(file) => file,
                Err(_) => {
                    return Ok(Response::new(Body::from("Well, that wasn't supposed to happen")))
                },
            };
            let response = response;
            (response, StatusCode::NOT_FOUND)}
        _ => panic!("status code not accounted for"),
    }; 
    let mut response :Response<Body> = Response::new(Body::from(response));
    *response.status_mut() = status;
    Ok(response)
} 

struct NonBlockingStd {
    list_files: Arc<Mutex<Vec<PathBuf>>>,
    folder: PathBuf,
}

impl NonBlockingStd {
    pub async fn new() -> Self {
        
        let args :Vec<String> = args().collect();
        let path = PathBuf::from(&args[1]);
        if args.len() != 2 || !path.is_dir() {
            eprintln!("ERROR: incorrect arguments");
            eprintln!("Please specify the relative path of a valid folder");
            eprintln!("Usage: ./website <folder>");
            exit(1)
        } 
        let list = Self::search_dir(&path).await;
        
        let list = Arc::new(Mutex::new(list));
        NonBlockingStd{ list_files: list , folder: path }

    }


    pub async fn stdin_loop(tx: Sender<()>) {
        // loop indefinetely and read from stdin in separate thread to avoid blocking server
        loop { 
            let mut buffer = String::new();
            io::stdin().read_line(&mut buffer).unwrap();

            match buffer.trim() {
               "exit"  => {

                    // if we need to terminate the program we send the signal
                    // and then break the loop since we don't need to iterate anymore
                    println!("Gracefully shutting down the server now");
                    tx.send(()).unwrap();
                    break;},

                _ => continue
            }
        }
    }
    pub async fn sync(info: Self)  -> Self {
        let dir = Arc::new(Mutex::new(Self::search_dir(&info.folder).await));
        NonBlockingStd { folder: info.folder, list_files: dir }
    }

    async fn search_dir(path: &PathBuf) -> Vec<PathBuf> {
         let mut dir = read_dir(&path).await.unwrap();
            let mut list:Vec<PathBuf> = Vec::new();
           
            while let Some(entry) = dir.next_entry().await.unwrap() {
                let path = entry.path();
                let string = &path.into_os_string().into_string().unwrap()
                    //reverse so that first "." must be the type of file
                    .trim().chars().rev().collect::<String>();
                let end = &string
                    .trim()
                    [..=string.find(".").unwrap()];
                // check for html or css
                if end == "lmth." || end == "ssc." {
                    //back to normal only if necessary
                    let string:String= string.chars().rev().collect();
                    list.push(PathBuf::from(string))
                }           
        }
    list
}
}
